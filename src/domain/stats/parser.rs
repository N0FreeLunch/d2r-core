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
    
    // Non-Normal Alpha v105 uses 10-bit property model (9-bit ID + 1-bit value).
    // EXCEPT for Runewords, which use the Retail variable-width model.
    let is_alpha_model = (version == 5 || version == 1) && !_alpha_runeword;
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    let stat_id = recorder.read_bits::<u32>(id_bits)?;
    crate::item_trace!("[DEBUG] [{}] Stat ID: {}", recorder.pos(), stat_id);
    
    if stat_id == terminator {
        let mut term_bit = false;
        if is_alpha_model {
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
    if is_alpha_model {
        // 1-bit value for non-Normal items.
        raw_value = recorder.read_bits::<u32>(1)? as u32;
    } else {
        // Retail logic for bit widths.
        let effective_id = if version == 5 || version == 1 {
            crate::domain::stats::lookup_alpha_map_by_raw(stat_id).map(|m| m.effective_id).unwrap_or(stat_id)
        } else {
            stat_id
        };

        if let Some(stat) = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == effective_id) {
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
