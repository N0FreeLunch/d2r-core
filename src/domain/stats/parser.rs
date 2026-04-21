use bitstream_io::BitRead;
use crate::domain::item::{ItemBitRange};
use crate::domain::stats::{
    ItemProperty,
};
use crate::data::bit_cursor::BitCursor;
use crate::item::{HuffmanTree, ParsingResult, PropertyReaderContext};

/// Coordinates the parsing of a property list, which is terminated by a 9-bit 0x1FF value.
pub fn read_property_list<'a, R: BitRead, F>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext<'a>,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
    mut recovery_fn: F,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)>
where
    F: FnMut(&[u8], u64, &HuffmanTree, usize, bool) -> ParsingResult<(crate::domain::item::Item, u64)>,
{
    let mut props = Vec::new();
    let mut saw_terminator = false;
    let mut terminator_bit = false;

    loop {
        let result = parse_single_property(recorder, code, version, &section_recovery, huffman, alpha_runeword, &mut recovery_fn);
        match result {
            Ok(Some((prop, is_term, term_bit))) => {
                if is_term {
                    saw_terminator = true;
                    terminator_bit = term_bit;
                    break;
                }
                props.push(prop);
            }
            Ok(None) => break,
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok((props, saw_terminator, terminator_bit))
}

/// Parses a single property entry from the bitstream.
pub fn parse_single_property<'a, R: BitRead, F>(
    recorder: &mut BitCursor<R>,
    _code: &str,
    version: u8,
    _section_recovery: &PropertyReaderContext<'a>,
    _huffman: &HuffmanTree,
    _alpha_runeword: bool,
    _recovery_fn: &mut F,
) -> ParsingResult<Option<(ItemProperty, bool, bool)>>
where
    F: FnMut(&[u8], u64, &HuffmanTree, usize, bool) -> ParsingResult<(crate::domain::item::Item, u64)>,
{
    let entry_start = recorder.pos();
    
    // Alpha v105 specific model check.
    let is_alpha_version = version == 5 || version == 1;
    let is_alpha_flag_model = is_alpha_version && !_alpha_runeword;
    
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    let stat_id = recorder.read_bits::<u32>(id_bits)?;
    crate::item_trace!("[DEBUG] [{}] Stat ID: {}", recorder.pos(), stat_id);
    
    if stat_id == terminator {
        let mut term_bit = false;
        if is_alpha_version {
            // Alpha v105 items (including runewords) often have a 1-bit terminal nudge.
            term_bit = recorder.read_bit()?;
        }
        return Ok(Some((
            ItemProperty {
                stat_id,
                raw_value: 0,
                param: 0,
                name: String::new(),
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
        // 1-bit value for non-Normal/non-Runeword Alpha items.
        raw_value = recorder.read_bits::<u32>(1)? as u32;
    } else if is_alpha_version {
        // Alpha v105 Runeword or complex model.
        if let Some(map) = crate::domain::stats::lookup_alpha_map_by_raw(stat_id) {
            if let Some(stat) = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == map.effective_id) {
                if stat.save_param_bits > 0 {
                    param = recorder.read_bits::<u32>(stat.save_param_bits as u32)?;
                }
                raw_value = recorder.read_bits::<u32>(stat.save_bits as u32)?;
            } else {
                raw_value = recorder.read_bits::<u32>(9)? as u32;
            }
        } else {
            // UNKNOWN Alpha Stat ID. Forensic evidence suggests 9-bit values are the standard fallback.
            raw_value = recorder.read_bits::<u32>(9)? as u32;
            crate::item_trace!("[DEBUG] [{}] UNKNOWN Alpha Stat ID {}, read 9-bit value: {}", recorder.pos(), stat_id, raw_value);
        }
    } else {
        // Retail logic for bit widths.
        if let Some(stat) = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == stat_id) {
            if stat.save_param_bits > 0 {
                param = recorder.read_bits::<u32>(stat.save_param_bits as u32)?;
            }
            raw_value = recorder.read_bits::<u32>(stat.save_bits as u32)?;
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
            range: ItemBitRange {
                start: entry_start,
                end: entry_end,
            },
        },
        false,
        false
    )))
}
