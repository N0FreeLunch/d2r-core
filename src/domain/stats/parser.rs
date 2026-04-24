use bitstream_io::BitRead;
use crate::domain::item::{ItemBitRange};
use crate::domain::stats::{
    ItemProperty,
};
use crate::data::bit_cursor::BitCursor;
use crate::item::{HuffmanTree, ParsingResult, PropertyReaderContext};
use crate::domain::header::entity::ItemSegmentType;

pub fn read_property_list<R: BitRead, F>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    _section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
    is_v105_shadow: bool,
    mut recovery_fn: F,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)> 
where 
    F: FnMut(&[u8], u64, &HuffmanTree, usize, bool) -> ParsingResult<(crate::domain::item::Item, u64)>
{
    let mut props = Vec::new();
    let mut terminator_bit = false;
    let mut saw_terminator = false;
    
    // Heuristic for compact items in Alpha
    let is_compact = code.trim().is_empty() || code.len() < 3;

    loop {
        let result = parse_single_property_internal(recorder, version, huffman, alpha_runeword, is_compact, is_v105_shadow, &mut recovery_fn)?;
        match result {
            Some((prop, is_term, term_bit)) => {
                if is_term {
                    saw_terminator = true;
                    terminator_bit = term_bit;
                    break;
                }
                props.push(prop);
            }
            None => break,
        }
    }

    Ok((props, saw_terminator, terminator_bit))
}

pub fn parse_single_property<R, F>(
    recorder: &mut BitCursor<R>,
    version: u8,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
    section_recovery: F,
) -> ParsingResult<Option<(ItemProperty, bool, bool)>>
where
    R: BitRead,
    F: FnMut(&[u8], u64, &HuffmanTree, usize, bool) -> ParsingResult<(crate::domain::item::Item, u64)>,
{
    parse_single_property_internal(recorder, version, huffman, alpha_runeword, false, false, section_recovery)
}

fn parse_single_property_internal<R, F>(
    recorder: &mut BitCursor<R>,
    version: u8,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
    is_compact: bool,
    is_v105_shadow: bool,
    mut _section_recovery: F,
) -> ParsingResult<Option<(ItemProperty, bool, bool)>>
where
    R: BitRead,
    F: FnMut(&[u8], u64, &HuffmanTree, usize, bool) -> ParsingResult<(crate::domain::item::Item, u64)>,
{
    let entry_start = recorder.pos();
    let alpha_mode = version == 5 || version == 1 || version == 4;
    let is_alpha_flag_model = alpha_mode && !alpha_runeword && version != 5;
    
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    let stat_id = recorder.read_bits::<u32>(id_bits)?;
    
    if stat_id == terminator {
        let mut term_bit = false;
        if alpha_mode {
            term_bit = recorder.read_bit()?;
            // Alpha v105 forensic: Mandatory extra terminal bit (usually 0) before alignment
            if version == 5 {
                let _extra = recorder.read_bit()?;
            }
            while recorder.pos() % 8 != 0 {
                let _p = recorder.read_bit()?;
            }
        }
        return Ok(Some((
            ItemProperty {
                stat_id,
                raw_value: 0,
                param: 0,
                name: "terminator".to_string(),
                value: 0,
                range: ItemBitRange { start: entry_start, end: recorder.pos() },
            },
            true,
            term_bit
        )));
    }

    let mut raw_value = 0;
    let mut param = 0;

    if is_alpha_flag_model {
        raw_value = recorder.read_bits::<u32>(9)?;
    } else if alpha_mode {
        let mapped_id = map_alpha_stat_id(stat_id as u16);
        
        let mut width = 9;
        let mut is_rhythm = false;
        if (alpha_runeword || version == 5) && !is_compact {
            // Alpha v105 / DLC forensic: FIXED 18-bit rhythm (9-bit ID + 9-bit Value)
            width = 9;
            is_rhythm = true;
        } else {
            if let Some(stat) = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == mapped_id as u32) {
                if stat.save_param_bits > 0 {
                    param = recorder.read_bits::<u32>(stat.save_param_bits as u32)?;
                }
                width = stat.save_bits as u32;
            } else {
                width = 9;
            }
        }
        raw_value = recorder.read_bits::<u32>(if (alpha_runeword || version == 5) && !is_compact { 9 } else { width })?;
    } else {
        if let Some(stat) = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == stat_id) {
            if stat.save_param_bits > 0 {
                param = recorder.read_bits::<u32>(stat.save_param_bits as u32)?;
            }
            raw_value = recorder.read_bits::<u32>(stat.save_bits as u32)?;
        } else {
            raw_value = recorder.read_bits::<u32>(9)?;
        }
    }

    let entry_end = recorder.pos();
    Ok(Some((
        ItemProperty {
            stat_id,
            raw_value: raw_value as i32,
            param,
            name: String::new(),
            value: 0,
            range: ItemBitRange { start: entry_start, end: entry_end },
        },
        false,
        false
    )))
}

/// Parses a single property entry from the bitstream.
fn map_alpha_stat_id(alpha_id: u16) -> u16 {
    match alpha_id {
        26 => 16,   // item_defense_percent
        312 => 72,  // item_durability
        207 => 73,  // item_maxdurability
        380 => 194, // item_indestructible
        25 => 194,  // item_numsockets_alpha
        256 => 127, // item_allskills
        496 => 99,  // item_fastergethitrate
        499 => 16,  // item_enandefense_percent
        289 => 9,   // maxmana
        _ => alpha_id,
    }
}
