use bitstream_io::BitRead;
use bitstream_io::{BitReader, LittleEndian};
use super::quality::ItemQuality;
use super::entity::ItemBitRange;
use crate::data::bit_cursor::BitCursor;
use crate::item::{HuffmanTree, ParsingResult, ParsingError, PropertyReaderContext};
use crate::data::stat_costs::STAT_COSTS;
use std::io::Cursor;

macro_rules! item_trace {
    ($($arg:tt)*) => {
        if crate::item::item_trace_enabled() {
            println!($($arg)*);
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStats {
    pub properties: Vec<ItemProperty>,
    pub set_attributes: Vec<Vec<ItemProperty>>,
    pub runeword_attributes: Vec<ItemProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemProperty {
    pub stat_id: u32,
    pub name: String,
    pub param: u32,
    pub raw_value: i32,
    pub value: i32, // After applying save_add if needed
    pub range: ItemBitRange,
}

#[derive(Debug, Clone, Copy)]
pub struct AlphaStatMap {
    pub raw_id: u32,
    pub effective_id: u32,
    pub name: &'static str,
}

pub const ALPHA_STAT_MAPS: &[AlphaStatMap] = &[
    AlphaStatMap { raw_id: 256, effective_id: 127, name: "item_allskills" },
    AlphaStatMap { raw_id: 496, effective_id: 99,  name: "item_fastergethitrate" },
    AlphaStatMap { raw_id: 499, effective_id: 16,  name: "item_enandefense_percent" },
    AlphaStatMap { raw_id: 289, effective_id: 9,   name: "maxmana" },
    AlphaStatMap { raw_id: 26,  effective_id: 31,  name: "item_defense_percent" },
    AlphaStatMap { raw_id: 312, effective_id: 72,  name: "item_durability" },
    AlphaStatMap { raw_id: 207, effective_id: 73,  name: "item_maxdurability" },
    AlphaStatMap { raw_id: 380, effective_id: 194, name: "item_indestructible" },
];

pub fn lookup_alpha_map_by_raw(raw_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.raw_id == raw_id)
}

pub fn lookup_alpha_map_by_effective(effective_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.effective_id == effective_id)
}

pub fn stat_save_bits(stat_id: u32) -> Option<u32> {
    STAT_COSTS
        .iter()
        .find(|stat| stat.id == stat_id)
        .map(|stat| stat.save_bits as u32)
}

pub enum PropertyParseResult {
    Property(ItemProperty),
    Terminator(bool),
    Recovered,
}

pub fn read_property_list<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)> {
    recorder.push_context("Property List");
    let mut props = Vec::new();

    let _is_alpha = version == 5 || version == 1;

    if version == 5 && alpha_runeword { 
        item_trace!("[DEBUG v5] Skipping 93-bit Alpha runeword spacer at bit {}", recorder.pos());
        for _ in 0..93 {
            let _ = recorder.read_bit()?;
        }
        item_trace!("[DEBUG v5] Starting List 2 at bit {}", recorder.pos()); 
    }
    loop {
        let result = parse_single_property(recorder, code, version, section_recovery, huffman, alpha_runeword);
        match result {
            Ok(PropertyParseResult::Property(prop)) => {
                item_trace!("  [Property] parsed ID={}, val={}" , prop.stat_id , prop.value);
                props.push(prop)
            },
            Ok(PropertyParseResult::Terminator(bit)) => {
                recorder.end_segment();
                return Ok((props, true, bit));
            },
            Ok(PropertyParseResult::Recovered) => {
                recorder.end_segment();
                return Ok((props, false, false));
            },
            Err(e) if version == 5 && matches!(e.error, ParsingError::Io(ref msg) if msg.contains("unexpected end of file")) => {
                item_trace!("  [Alpha v5] Property list reached EOF without terminator.");
                recorder.end_segment();
                return Ok((props, false, false));
            }
            Err(e) => {
                recorder.end_segment();
                return Err(e);
            },
        }
    }
}

pub fn parse_single_property<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    _alpha_runeword: bool,
) -> ParsingResult<PropertyParseResult> {
    recorder.push_context("Single Property");
    let start_bit = recorder.pos();

    if version == 5 || version == 1 {
        let stat_id = match read_alpha_stat_id(recorder) {
            Ok(id) => id,
            Err(_) if version == 5 => {
                item_trace!("[DEBUG v5] Property stream ended abruptly, assuming terminator.");
                recorder.end_segment();
                return Ok(PropertyParseResult::Terminator(false));
            }
            Err(e) => {
                recorder.end_segment();
                return Err(e);
            },
        };
        
        if version == 5 {
             item_trace!("[DEBUG v5] Property Stat ID: {} (0x{:03X}) at {}", stat_id, stat_id, recorder.pos() - 9);
        }
        
        if is_alpha_terminator(stat_id, code) {
            if version == 5 {
                 item_trace!("[DEBUG v5] Property Terminator detected at {} for '{}'", recorder.pos() - 9, code);
            }
            let mut term_bit = false;
            if recorder.alpha_quality != Some(ItemQuality::Normal) {
                let _ = recorder.read_bit()?; // Optional 10th bit
            }
            recorder.end_segment();
            return Ok(PropertyParseResult::Terminator(term_bit));
        }

        let (effective_stat_id, stat_name, save_add) = if let Some(m) = lookup_alpha_map_by_raw(stat_id) {
             let cost = STAT_COSTS.iter().find(|s| s.id == m.effective_id);
             (m.effective_id, m.name.to_string(), cost.map(|c| c.save_add).unwrap_or(0))
        } else {
             let cost = STAT_COSTS.iter().find(|s| s.id == stat_id);
             (stat_id, cost.map(|c| c.name.to_string()).unwrap_or_else(|| format!("alpha_stat_{}", stat_id)), cost.map(|c| c.save_add).unwrap_or(0))
        };

        // Alpha v105 Quality-dependent property widths:
        // Normal items use 9 bits (ID only).
        // Others (Magic/Rare/Unique) use 10 bits (9 ID + 1 Val).
        let val_bits = if recorder.alpha_quality == Some(ItemQuality::Normal) { 0 } else { 1 };
        let val = if val_bits > 0 { recorder.read_bits::<u32>(val_bits)? } else { 0 };
        
        if version == 5 {
             item_trace!("[DEBUG v5] Property ID {} Value: {} at {} ({}-bit quality-based)", stat_id, val, recorder.pos() - val_bits as u64, 9 + val_bits);
        }

        let end_bit = recorder.pos();
        recorder.end_segment();

        return Ok(PropertyParseResult::Property(ItemProperty {
            stat_id: effective_stat_id,
            name: stat_name,
            param: 0,
            raw_value: val as i32,
            value: (val as i32).wrapping_sub(save_add),
            range: ItemBitRange { start: start_bit, end: end_bit },
        }));
    }

    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    let stat_id = match recorder.read_bits::<u32>(id_bits) {
        Ok(stat_id) => stat_id,
        Err(err) => {
            if let Some((section_bytes, item_start_bit)) = section_recovery {
                if crate::item::recover_property_reader(recorder, code, section_bytes, item_start_bit, huffman)? {
                    recorder.end_segment();
                    return Ok(PropertyParseResult::Recovered);
                }
            }
            recorder.end_segment();
            return Err(err);
        }
    };

    if stat_id == terminator {
        recorder.end_segment();
        return Ok(PropertyParseResult::Terminator(false));
    }

    let (effective_stat_id, save_bits, save_add, stat_name) = {
        let cost = STAT_COSTS.iter().find(|s| s.id == stat_id).ok_or_else(|| {
             recorder.fail(ParsingError::InvalidStatId { bit_offset: recorder.pos(), stat_id })
        })?;
        (stat_id, cost.save_bits as u32, cost.save_add, cost.name.to_string())
    };

    let val = recorder.read_bits::<u32>(save_bits)?;
    let end_bit = recorder.pos();
    recorder.end_segment();

    Ok(PropertyParseResult::Property(ItemProperty {
        stat_id: effective_stat_id,
        name: stat_name,
        param: 0,
        raw_value: val as i32,
        value: (val as i32) - save_add,
        range: ItemBitRange { start: start_bit, end: end_bit },
    }))
}

fn read_alpha_stat_id<R: BitRead>(recorder: &mut BitCursor<R>) -> ParsingResult<u32> {
    recorder.read_bits::<u32>(9)
}

fn is_alpha_terminator(stat_id: u32, code: &str) -> bool {
    if code.trim() == "hp1" {
        stat_id == 0x000 || stat_id == 0x1FF
    } else {
        stat_id == 0x1FF
    }
}

pub fn recover_alpha_xrs_properties(
    section_bytes: &[u8],
    item_start_bit: u64,
) -> Vec<ItemProperty> {
    const LIST1_OFFSET_BITS: u64 = 129;
    const MAX_PROPERTY_SLOTS: usize = 16;

    let anchor_bit = item_start_bit + LIST1_OFFSET_BITS;
    let mut reader = BitReader::endian(Cursor::new(section_bytes), LittleEndian);
    if reader.skip(anchor_bit as u32).is_err() {
        return Vec::new();
    }

    let mut recorder = BitCursor::new(reader);
    let mut props = Vec::new();
    let mut saw_terminator = false;

    for _ in 0..MAX_PROPERTY_SLOTS {
        let entry_start = anchor_bit + recorder.pos();
        let raw_id = match recorder.read_bits::<u32>(9) {
            Ok(id) => id,
            _ => break,
        };

        if raw_id == 0x1FF {
            if recorder.read_bits::<u32>(8).is_ok() {
                saw_terminator = true;
            }
            break;
        }

        let raw_value_bits = match recorder.read_bits::<u32>(8) {
            Ok(v) => v,
            _ => break,
        };
        let raw_value = raw_value_bits as i32;

        let (effective_id, name, save_add) = if let Some(map) = lookup_alpha_map_by_raw(raw_id) {
            let cost = STAT_COSTS.iter().find(|stat| stat.id == map.effective_id);
            (
                map.effective_id,
                map.name.to_string(),
                cost.map(|stat| stat.save_add).unwrap_or(0),
            )
        } else {
            let cost = STAT_COSTS.iter().find(|stat| stat.id == raw_id);
            (
                raw_id,
                cost.map(|stat| stat.name.to_string())
                    .unwrap_or_else(|| format!("alpha_stat_{}", raw_id)),
                cost.map(|stat| stat.save_add).unwrap_or(0),
            )
        };

        let entry_end = anchor_bit + recorder.pos();
        props.push(ItemProperty {
            stat_id: effective_id,
            name,
            param: 0,
            raw_value,
            value: raw_value.wrapping_sub(save_add),
            range: ItemBitRange {
                start: entry_start,
                end: entry_end,
            },
        });
    }

    if saw_terminator && !props.is_empty() {
        props
    } else {
        Vec::new()
    }
}
