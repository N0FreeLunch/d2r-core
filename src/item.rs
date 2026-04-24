use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use std::io::{self, Cursor};
use crate::data::bit_cursor::BitCursor;

pub(crate) fn item_trace_enabled() -> bool {
    std::env::var_os("D2R_ITEM_TRACE").is_some()
}

#[macro_export]
macro_rules! item_trace {
    ($($arg:tt)*) => {
        if crate::item::item_trace_enabled() {
            println!($($arg)*);
        }
    };
}

pub use crate::domain::item::{Item, ItemQuality, ItemBitRange, RecordedBit, ItemModule, BitSegment};
pub use crate::domain::header::entity::ItemSegmentType;
pub use crate::domain::item::serialization::HuffmanTree;
pub use crate::error::{ParsingError, ParsingFailure, ParsingResult};
pub use crate::domain::stats::ItemProperty;
use crate::domain::stats::{read_property_list, stat_save_bits};

#[derive(Debug, Clone)]
pub struct PropertyReaderContext<'a> {
    pub bytes: &'a [u8],
    pub item_start_bit: u64,
}

impl Item {
    pub fn read_player_items(bytes: &[u8], huffman: &HuffmanTree, alpha: bool) -> ParsingResult<Vec<Item>> {
        let mut best_jm_pos = None;
        let mut backup_jm_pos = None;

        for i in 0..bytes.len().saturating_sub(3) {
            if bytes[i] == b'J' && bytes[i + 1] == b'M' {
                let count = u16::from_le_bytes([bytes[i + 2], bytes[i + 3]]);
                
                if count == 0 {
                    if backup_jm_pos.is_none() {
                        backup_jm_pos = Some(i);
                    }
                } else {
                    // Peek at the first item to see if it's plausible.
                    let section_payload = &bytes[i + 4..];
                    if let Some((mode, location, _x, code, flags, version, _is_compact, _header_bits, _nudge)) = 
                        peek_item_header_at(section_payload, 0, huffman, alpha) 
                    {
                        if is_plausible_item_header(mode, location, &code, flags, version, alpha) {
                            best_jm_pos = Some(i);
                            break;
                        }
                    }
                    
                    if backup_jm_pos.is_none() {
                        backup_jm_pos = Some(i);
                    }
                }
            }
        }

        let start_offset = best_jm_pos.or(backup_jm_pos).ok_or_else(|| ParsingFailure {
            error: ParsingError::MissingMarker { marker: "JM".to_string(), bit_offset: 0 },
            context_stack: vec!["read_player_items".to_string()],
            bit_offset: 0,
            context_relative_offset: 0,
            hint: Some("Could not find a valid start of item section.".to_string()),
        })?;

        item_trace!("[DEBUG] Selected JM section at offset {}", start_offset);

        if bytes.len() < start_offset + 4 {
            return Err(ParsingFailure {
                error: ParsingError::MissingMarker { marker: "JM count".to_string(), bit_offset: (start_offset * 8) as u64 },
                context_stack: vec!["read_player_items".to_string()],
                bit_offset: (start_offset * 8) as u64,
                context_relative_offset: 0,
                hint: Some("Item section header (JM count) is incomplete.".to_string()),
            });
        }

        let count = u16::from_le_bytes([bytes[start_offset + 2], bytes[start_offset + 3]]);
        
        let payload_start = start_offset + 4;

        if bytes.len() < payload_start {
             return Err(ParsingFailure {
                error: ParsingError::Io("Section too short for preamble".to_string()),
                context_stack: vec!["read_player_items".to_string()],
                bit_offset: (payload_start * 8) as u64,
                context_relative_offset: 0,
                hint: Some("The item section ended before the expected preamble.".to_string()),
            });
        }

        let section_bytes = &bytes[payload_start..];
        Self::read_section(section_bytes, count, huffman, alpha)
    }

    pub fn read_section(section_bytes: &[u8], top_level_count: u16, huffman: &HuffmanTree, alpha_mode: bool) -> ParsingResult<Vec<Item>> {
        let mut items: Vec<Item> = Vec::new();
        let mut bit_pos = 0;
        let section_bits = (section_bytes.len() * 8) as u64;

        while items.len() < top_level_count as usize && bit_pos < section_bits {
            let start = if alpha_mode {
                find_next_item_match(section_bytes, bit_pos, huffman, alpha_mode).unwrap_or(bit_pos)
            } else {
                bit_pos
            };
            item_trace!("[DEBUG] read_section: Found item candidate at bit {}", start);
            
            // Alpha v105 Forensic: Use Lookahead to set a strict bit limit for the item.
            let next_item_start = if alpha_mode {
                section_bits // Disable lookahead for Alpha to avoid ghost items in flags
            } else {
                section_bits
            };
            let strict_limit = next_item_start - start;
            match parse_item_at_with_limit(section_bytes, start, huffman, items.len(), alpha_mode, Some(strict_limit)) {
                Ok((item, mut consumed_bits)) => {
                    if alpha_mode {
                        // Alpha v105 Forensic: All items are byte-aligned.
                        // Compact items (like potions) are exactly 80 bits (10 bytes).
                        if item.is_compact {
                            if consumed_bits < 80 {
                                consumed_bits = 80;
                            }
                        }
                        if consumed_bits % 8 != 0 {
                            consumed_bits += 8 - (consumed_bits % 8);
                        }
                    }

                    let mut end = start + consumed_bits;
                    let mut final_item = item;
                    final_item.range.end = end;
                    final_item.total_bits = consumed_bits;

                    item_trace!("[DEBUG] read_section: Item {} ({}) consumed {} bits", items.len(), final_item.code.trim(), consumed_bits);


                    
                    bit_pos = end;
                    items.push(final_item);
                }
                Err(e) => {
                    item_trace!("[DEBUG] read_section: Failed to parse item at bit {}: {:?}", start, e);
                    if alpha_mode {
                        if let Some(next_real_start) = find_next_item_match(section_bytes, start + 8, huffman, alpha_mode) {
                            item_trace!("[DEBUG] Alpha Rescue (Error): Skipping to next item at bit {}", next_real_start);
                            bit_pos = next_real_start;
                            continue;
                        }
                    }
                    
                    if let ParsingError::Io(ref s) = e.error {
                        if s.contains("Bit limit exceeded") || s.contains("unexpected end of file") { break; }
                    }
                    if alpha_mode { bit_pos = start + 8; } else { return Err(e); }
                }
            }
        }
        Ok(items)
    }

    pub fn from_bytes(bytes: &[u8], huffman: &HuffmanTree, alpha: bool) -> ParsingResult<Item> {
        let (item, _) = parse_item_at_with_limit(bytes, 0, huffman, 0, alpha, None)?;
        Ok(item)
    }

    pub fn from_reader<R: BitRead>(
        reader: &mut R,
        huffman: &HuffmanTree,
        alpha: bool,
    ) -> ParsingResult<Item> {
        let mut cursor = BitCursor::new(reader);
        Self::from_reader_with_context(&mut cursor, huffman, None, alpha)
    }

    pub fn from_reader_with_context<R: BitRead>(
        cursor: &mut BitCursor<R>,
        huff: &HuffmanTree,
        ctx: Option<(&[u8], u64)>,
        alpha_mode: bool,
    ) -> ParsingResult<Item> {
        cursor.set_trace(item_trace_enabled());
        let start_bit = cursor.pos();
        cursor.begin_segment(ItemSegmentType::Root);

        let flags = cursor.read_bits::<u32>(32)?;
        if !alpha_mode && (flags & 0xFFFF) != 0x4D4A {
             return Err(cursor.fail(ParsingError::MissingMarker { marker: "JM".to_string(), bit_offset: start_bit }));
        }

        let version = cursor.read_bits::<u8>(3)? as u8;
        let mode = cursor.read_bits::<u8>(3)? as u8;
        let location = cursor.read_bits::<u8>(3)? as u8;
        let x = cursor.read_bits::<u8>(4)? as u8;
        
        let is_compact = (flags & (1 << 21)) != 0;
        let mut y = 0;
        let mut page = 0;
        let mut header_socket_hint = 0;

        if alpha_mode && version == 5 {
            y = cursor.read_bits::<u8>(4)? as u8;
            page = cursor.read_bits::<u8>(3)? as u8;
            header_socket_hint = cursor.read_bits::<u8>(1)? as u8;
            cursor.read_bits::<u8>(8)?; // Alpha v105 Version 5 Header Gap
        } else if alpha_mode && (version == 1 || version == 4) {
            y = cursor.read_bits::<u8>(4)? as u8;
            page = cursor.read_bits::<u8>(3)? as u8;
            header_socket_hint = cursor.read_bits::<u8>(3)? as u8;
            cursor.read_bits::<u8>(8)?; // Alpha v105 Version 1 Header Gap
        } else if !is_compact {
            y = cursor.read_bits::<u8>(4)? as u8;
            page = cursor.read_bits::<u8>(3)? as u8;
            header_socket_hint = cursor.read_bits::<u8>(3)? as u8;
        }

        let mut code = String::new();
        let is_ear = (flags & (1 << 24)) != 0;
        let (mut ear_class, mut ear_level, mut ear_player_name) = (None, None, None);

        if is_ear {
            ear_class = Some(cursor.read_bits::<u8>(3)? as u8);
            ear_level = Some(cursor.read_bits::<u8>(7)? as u8);
            ear_player_name = Some(read_player_name(cursor)?);
        } else {
            for _ in 0..4 {
                code.push(huff.decode_recorded(cursor)?);
            }
        }

        let is_frag = alpha_mode && (version == 5 || version == 1) && ((flags & (1 << 26)) != 0 || (flags & (1 << 27)) != 0);

        let stats = if !is_compact || (alpha_mode && version == 5) {
            let socket_flag = if alpha_mode && version != 5 { (flags & (1 << 27)) != 0 } else { (flags & (1 << 11)) != 0 };
            
            // Alpha v105 Runeword Heuristic
            let is_base_rw = alpha_mode && (version == 5 || version == 1) && !is_frag && 
                ((flags & (1 << 11)) != 0 || (flags & (1 << 12)) != 0 || (flags & (1 << 13)) != 0 || (flags & 0x800) != 0);
            
            let runeword_flag = if alpha_mode && version != 5 { (flags & (1 << 11)) != 0 } 
                else if alpha_mode && version == 5 { is_base_rw || is_frag }
                else { (flags & (1 << 26)) != 0 };
                
            Self::read_extended_stats(cursor, &code, is_compact, socket_flag, runeword_flag, (flags & (1 << 25)) != 0, version, alpha_mode, is_frag)?
        } else {
            (None, None, None, false, None, false, None, None, None, None, None, None, [None; 6], None, None, None, None, None, false, None, None, None, None, None, 0)
        };

        let end_bit = cursor.pos();
        let item = Item {
            bits: cursor.recorded_bits().to_vec(),
            flags, version, mode, location, x, y, page, header_socket_hint,
            is_ear, ear_class, ear_level, ear_player_name,
            code: code.clone(), is_compact,
            id: stats.0, level: stats.1, quality: stats.2,
            has_multiple_graphics: stats.3, multi_graphics_bits: stats.4,
            has_class_specific_data: stats.5, class_specific_bits: stats.6,
            low_high_graphic_bits: stats.7, magic_prefix: stats.8, magic_suffix: stats.9,
            rare_name_1: stats.10, rare_name_2: stats.11, rare_affixes: stats.12,
            unique_id: stats.13, runeword_id: stats.14, runeword_level: stats.15,
            personalized_player_name: stats.16, tbk_ibk_teleport: stats.17,
            timestamp_flag: stats.18, defense: stats.19, max_durability: stats.20,
            current_durability: stats.21, quantity: stats.22, sockets: stats.23,
            set_list_count: stats.24,
            num_socketed_items: 0,
            socketed_items: Vec::new(),
            range: ItemBitRange { start: start_bit, end: end_bit },
            total_bits: end_bit - start_bit,
            gap_bits: Vec::new(),
            terminator_bit: false,
            segments: cursor.segments().to_vec(),
            runeword_attributes: Vec::new(),
            set_attributes: Vec::new(),
            properties: Vec::new(),
            is_identified: (flags & (1 << 4)) != 0,
            is_socketed: {
                let socketed = if alpha_mode && version == 5 {
                    !is_compact && ((flags & (1 << 23)) != 0 || (flags & (1 << 11)) != 0)
                } else if alpha_mode {
                    (flags & (1 << 27)) != 0
                } else {
                    (flags & (1 << 11)) != 0
                };
                crate::item_trace!("[DEBUG] is_socketed logic: alpha={}, version={}, flags=0x{:08X}, result={}", alpha_mode, version, flags, socketed);
                socketed
            },
            is_personalized: (flags & (1 << 25)) != 0,
            is_runeword: {
                let rw = if alpha_mode && (version == 5 || version == 1) {
                    let is_frag = (flags & (1 << 26)) != 0 || (flags & (1 << 27)) != 0;
                    !is_frag && ((flags & (1 << 11)) != 0 || (flags & (1 << 12)) != 0 || (flags & (1 << 13)) != 0 || (flags & 0x800) != 0)
                } else if alpha_mode {
                    (flags & (1 << 11)) != 0
                } else {
                    (flags & (1 << 26)) != 0
                };
                crate::item_trace!("[DEBUG] is_runeword logic: alpha={}, version={}, flags=0x{:08X}, result={}", alpha_mode, version, flags, rw);
                rw
            },
            is_ethereal: (flags & (1 << 22)) != 0,
            properties_complete: false,
            modules: Vec::new(),
        };

        let mut final_item = item;
        if !is_compact {
            let is_v105_shadow = alpha_mode && final_item.version == 5 && (final_item.flags & (1 << 26)) != 0;
            let (props, complete, term) = read_item_stats(cursor, &final_item.code, final_item.version, ctx, huff, alpha_mode, final_item.quality, final_item.is_runeword, is_v105_shadow)?;
            final_item.properties = props;
            final_item.properties_complete = complete;
            final_item.terminator_bit = term;
        }

        cursor.end_segment();
        Ok(final_item)
    }

    fn read_extended_stats<R: BitRead>(
        cursor: &mut BitCursor<R>,
        code: &str,
        is_compact: bool,
        is_socketed: bool,
        is_runeword: bool,
        is_personalized: bool,
        version: u8,
        alpha_mode: bool,
        is_fragment: bool,
    ) -> ParsingResult<(
        Option<u32>, Option<u8>, Option<ItemQuality>,
        bool, Option<u8>, bool, Option<u16>,
        Option<u8>, Option<u16>, Option<u16>,
        Option<u8>, Option<u8>, [Option<u16>; 6],
        Option<u16>, Option<u16>, Option<u8>,
        Option<String>, Option<u8>, bool,
        Option<u32>, Option<u32>, Option<u32>, Option<u32>,
        Option<u8>, u8,
    )> {
        cursor.begin_segment(ItemSegmentType::ExtendedStats);
        let trimmed_code = code.trim();
        let is_alpha = alpha_mode && (version == 1 || version == 4);

        if alpha_mode && version == 5 && (is_runeword || is_fragment) {
            // Alpha v105 Runeword components skip directly to properties.
            cursor.end_segment();
            return Ok((
                Some(0u32), None, Some(ItemQuality::Normal),
                false, None, false, None,
                None, None, None, None, None, [None; 6],
                None, None, None, None, None, false,
                None, None, None, None, None, 0
            ));
        }

        let (item_id, item_level, item_quality, has_multiple_graphics, has_class_specific_data, timestamp_flag) = if alpha_mode && (version == 5 || version == 1 || version == 4) {
            if !is_compact {
                // Alpha v105: ID and Level are omitted, but Quality (3 bits) might be present.
                let quality_raw = cursor.read_bits::<u8>(3)?;
                let quality = ItemQuality::from(quality_raw);
                (Some(0u32), None, Some(quality), false, false, false)
            } else {
                (Some(0u32), None, None, false, false, false)
            }
        } else {
            let id = cursor.read_bits::<u32>(32)?;
            let level = cursor.read_bits::<u8>(7)?;
            let quality_raw = cursor.read_bits::<u8>(4)?;
            let quality = ItemQuality::from(quality_raw);
            (Some(id), Some(level), Some(quality), false, false, false)
        };

        if is_alpha {
            cursor.end_segment();
            return Ok((
                item_id, item_level, item_quality,
                false, None, false, None,
                None, None, None, None, None, [None; 6],
                None, None, None, None, None, timestamp_flag,
                None, None, None, None, None, 0
            ));
        }

        let mut multi_graphics_bits = None;
        let mut class_specific_bits = None;
        if has_multiple_graphics { multi_graphics_bits = Some(cursor.read_bits::<u8>(3)? as u8); }
        if has_class_specific_data { class_specific_bits = Some(cursor.read_bits::<u16>(11)? as u16); }
        
        let quality_val = item_quality.unwrap_or(ItemQuality::Normal);
        let (mut low_high_graphic_bits, mut magic_prefix, mut magic_suffix) = (None, None, None);
        let (mut rare_name_1, mut rare_name_2) = (None, None);
        let mut rare_affixes = [None; 6];
        let mut unique_id = None;

        match quality_val {
            ItemQuality::Low | ItemQuality::High => { low_high_graphic_bits = Some(cursor.read_bits::<u8>(3)? as u8); }
            ItemQuality::Magic => {
                magic_prefix = Some(cursor.read_bits::<u16>(11)? as u16);
                magic_suffix = Some(cursor.read_bits::<u16>(11)? as u16);
            }
            ItemQuality::Rare | ItemQuality::Crafted => {
                rare_name_1 = Some(cursor.read_bits::<u8>(8)? as u8);
                rare_name_2 = Some(cursor.read_bits::<u8>(8)? as u8);
                for i in 0..6 { if cursor.read_bit()? { rare_affixes[i] = Some(cursor.read_bits::<u16>(11)? as u16); } }
            }
            ItemQuality::Set | ItemQuality::Unique => { unique_id = Some(cursor.read_bits::<u16>(12)? as u16); }
            _ => {}
        }

        let (mut runeword_id, mut runeword_level) = (None, None);
        if is_runeword && !is_fragment && version != 5 {
            // Alpha v105: Runeword ID and Level are not in the extended header.
            runeword_id = Some(cursor.read_bits::<u16>(12)? as u16);
            runeword_level = Some(cursor.read_bits::<u8>(4)? as u8);
        }

        let mut personalized_player_name = None;
        if is_personalized { personalized_player_name = Some(read_player_name(cursor)?); }

        let tbk_ibk_teleport = if trimmed_code == "tbk" || trimmed_code == "ibk" { Some(cursor.read_bits::<u8>(5)? as u8) } else { None };
        let timestamp_flag = cursor.read_bit()?;

        let template = item_template(trimmed_code);
        let (reads_defense, reads_durability, reads_quantity) = if let Some(template) = template {
            (template.is_armor, template.has_durability, template.is_stackable)
        } else {
            let armor_like_unknown = has_class_specific_data || trimmed_code.contains(' ');
            (armor_like_unknown, armor_like_unknown, false)
        };

        let (mut defense, mut max_durability, mut current_durability, mut quantity, mut sockets) = (None, None, None, None, None);
        if reads_defense && (!alpha_mode && version != 5) { defense = Some(cursor.read_bits::<u32>(stat_save_bits(31).unwrap_or(11))?); }
        if reads_durability && (!alpha_mode && version != 5) {
            let max_bits = stat_save_bits(73).unwrap_or(8);
            let cur_bits = stat_save_bits(72).unwrap_or(9);
            let m_dur = cursor.read_bits::<u32>(max_bits)?;
            max_durability = Some(m_dur);
            if m_dur > 0 { current_durability = Some(cursor.read_bits::<u32>(cur_bits)?); let _extra = cursor.read_bit()?; }
        }
        if reads_quantity && (!alpha_mode && version != 5) { quantity = Some(cursor.read_bits::<u32>(9)?); }
        if is_socketed && (!alpha_mode || version == 5) { sockets = Some(cursor.read_bits::<u8>(4)? as u8); }

        cursor.end_segment();
        Ok((
            item_id, item_level, item_quality,
            has_multiple_graphics, multi_graphics_bits,
            has_class_specific_data, class_specific_bits,
            low_high_graphic_bits, magic_prefix, magic_suffix,
            rare_name_1, rare_name_2, rare_affixes,
            unique_id, runeword_id, runeword_level,
            personalized_player_name, tbk_ibk_teleport,
            timestamp_flag, defense, max_durability, current_durability, quantity, sockets, 0
        ))
    }
}

fn read_item_stats<R: BitRead>(
    cursor: &mut BitCursor<R>,
    code: &str,
    version: u8,
    ctx: Option<(&[u8], u64)>,
    huffman: &HuffmanTree,
    alpha_mode: bool,
    quality: Option<ItemQuality>,
    is_runeword: bool,
    is_v105_shadow: bool,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)> {
    cursor.begin_segment(ItemSegmentType::Stats);
    let trimmed_code = code.trim();
    let is_alpha = alpha_mode && (version == 5 || version == 1 || version == 4);
    let quality_val = quality.unwrap_or(ItemQuality::Normal);
    crate::item_trace!("[DEBUG] read_item_stats for '{}', version={}, is_runeword={}, quality={:?}, is_alpha={}", trimmed_code, version, is_runeword, quality, is_alpha);

    let is_v105_shadow = alpha_mode && version == 5 && is_v105_shadow;
    if is_alpha && version == 5 && !is_v105_shadow && !is_runeword {
         crate::item_trace!("[DEBUG] Skipping properties for Alpha v105 Summary Item '{}'", trimmed_code);
         return Ok((Vec::new(), true, false));
    }
    
    if is_alpha && version != 5 && (quality_val == ItemQuality::Normal || trimmed_code == "hp1") && !is_runeword && !trimmed_code.is_empty() {
         crate::item_trace!("[DEBUG] Skipping properties for Alpha Item '{}' (v{})", trimmed_code, version);
         return Ok((Vec::new(), true, false));
    }
    
    let section_recovery = if let Some((bytes, start)) = ctx {
        PropertyReaderContext { bytes, item_start_bit: start }
    } else {
        PropertyReaderContext { bytes: &[], item_start_bit: 0 }
    };

    read_property_list(cursor, trimmed_code, version, section_recovery, huffman, is_runeword, is_v105_shadow, |_, _, _, _, _| {
        let r = IoBitReader::endian(Cursor::new(&[]), LittleEndian);
        let mut c = BitCursor::new(r);
        let d = Item::from_reader_with_context(&mut c, huffman, None, alpha_mode)?;
        Ok((d, 0))
    })
}

fn read_player_name<R: BitRead>(cursor: &mut BitCursor<R>) -> ParsingResult<String> {
    let mut name = String::new();
    loop {
        let ch = cursor.read_bits::<u8>(7)?;
        if ch == 0 { break; }
        name.push(ch as char);
    }
    Ok(name)
}

fn item_template(code: &str) -> Option<&'static crate::data::item_codes::ItemTemplate> {
    crate::data::item_codes::ITEM_TEMPLATES.iter().find(|t| t.code == code.trim())
}

pub fn find_next_item_match(bytes: &[u8], pos: u64, huffman: &HuffmanTree, alpha: bool) -> Option<u64> {
    let limit = (bytes.len() * 8) as u64;
    let mut probe = pos;
    while probe < limit {
        // Alpha v105 items are strictly byte-aligned.
        if alpha && probe % 8 != 0 {
            probe += 1;
            continue;
        }
        if let Some((mode, location, _x, code, flags, version, _is_compact, _header_bits, _nudge)) = peek_item_header_at(bytes, probe, huffman, alpha) {
            if is_plausible_item_header(mode, location, &code, flags, version, alpha) {
                return Some(probe);
            }
        }
        probe += 1;
    }
    None
}

pub fn is_plausible_item_header(
    mode: u8,
    location: u8,
    code: &str,
    flags: u32,
    version: u8,
    alpha_mode: bool,
) -> bool {
    crate::item_trace!("[DEBUG] plausible check: code='{}', mode={}, loc={}, flags=0x{:08X}, v={}, alpha={}", code, mode, location, flags, version, alpha_mode);
    if code.len() < 3 { return false; }
    if !code.chars().all(|c| c.is_alphanumeric() || c == ' ') { return false; }
    if alpha_mode {
        // Alpha v105 codes are usually 4 chars (e.g. '7pww', 'hp1 ', '1pww').
        if code.len() != 4 { return false; }
        if !code.chars().all(|c| c.is_alphanumeric() || c == ' ') { return false; }
        if code.trim().is_empty() { return false; }
        
        if mode > 6 || location > 15 { return false; }
        // The top 5 bits of flags are used in Alpha (e.g. 0x6B... for Authority).
    } else if mode > 6 || location > 15 { return false; }
    true
}

pub fn peek_item_header_at(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
    alpha_mode: bool,
) -> Option<(u8, u8, u8, String, u32, u8, bool, u64, i8)> {
    let mut reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
    if reader.skip(start_bit as u32).is_err() { return None; }

    let flags = reader.read::<32, u32>().ok()?;
    if !alpha_mode && (flags & 0xFFFF) != 0x4D4A {
        return None;
    }
    let version = reader.read::<3, u8>().ok()?;
    let mode = reader.read::<3, u8>().ok()?;
    let loc = reader.read::<3, u8>().ok()?;
    let x = reader.read::<4, u8>().ok()?;
    
    let is_compact = (flags & (1 << 21)) != 0;
    let mut header_len = 32 + 3 + 3 + 3 + 4; // flags(32) + v(3) + m(3) + l(3) + x(4)
    
    if alpha_mode && version == 5 {
        header_len += 16; // 4 (y) + 3 (page) + 1 (hint) + 8 (gap)
    } else if alpha_mode && (version == 1 || version == 4) {
        header_len += 18; // 4 (y) + 3 (page) + 3 (hint) + 8 (gap)
    } else if !is_compact {
        header_len += 10; // 4 (y) + 3 (page) + 3 (hint)
    }

    let mut code = String::new();
    let mut n_reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
    if n_reader.skip(start_bit as u32 + header_len as u32).is_err() { return None; }
    let mut n_cursor = BitCursor::new(n_reader);
    
    for _ in 0..4 {
        if let Ok(ch) = huffman.decode_recorded(&mut n_cursor) { code.push(ch); }
        else { return None; }
    }
    
    if is_plausible_item_header(mode, loc, &code, flags, version, alpha_mode) {
        return Some((mode, loc, x, code, flags, version, is_compact, header_len as u64, 0));
    }
    None
}

pub fn parse_item_at(
    bytes: &[u8],
    bit: u64,
    huffman: &HuffmanTree,
    idx: usize,
    alpha: bool,
) -> ParsingResult<(Item, u64)> {
    parse_item_at_with_limit(bytes, bit, huffman, idx, alpha, None)
}

pub fn parse_item_at_with_limit(
    bytes: &[u8],
    bit: u64,
    huffman: &HuffmanTree,
    _idx: usize,
    alpha: bool,
    limit: Option<u64>,
) -> ParsingResult<(Item, u64)> {
    let mut reader = IoBitReader::endian(Cursor::new(bytes), LittleEndian);
    let _ = reader.skip(bit as u32);
    let mut cursor = BitCursor::new(reader);
    if let Some(l) = limit {
        cursor.set_limit(l);
    }
    let item = Item::from_reader_with_context(&mut cursor, huffman, Some((bytes, bit)), alpha)?;
    Ok((item, cursor.pos()))
}

fn scan_socket_children(
    bytes: &[u8],
    bit_pos: u64,
    huffman: &HuffmanTree,
    _parent_idx: usize,
    alpha: bool,
    limit: u64,
) -> Option<(Vec<Item>, u64)> {
    let mut children = Vec::new();
    let mut current_pos = bit_pos;
    let max_pos = bit_pos + limit;
    let section_bits = (bytes.len() * 8) as u64;

    while current_pos < max_pos && current_pos < section_bits {
        if let Some((mode, location, _x, code, flags, version, _is_compact, _header_bits, _nudge)) = peek_item_header_at(bytes, current_pos, huffman, alpha) {
            if is_plausible_item_header(mode, location, &code, flags, version, alpha) {
                if mode == 6 || location == 6 {
                    let remaining = section_bits.saturating_sub(current_pos);
                    if let Ok((item, consumed)) = parse_item_at_with_limit(bytes, current_pos, huffman, 0, alpha, Some(remaining)) {
                        let mut item_end = current_pos + consumed;
                        if alpha {
                            if let Some(next_start) = find_next_item_match(bytes, current_pos + 64, huffman, alpha) {
                                if next_start < item_end && next_start < max_pos { item_end = next_start; }
                            }
                        }
                        let mut final_child = item;
                        final_child.range.start = current_pos;
                        final_child.range.end = item_end;
                        final_child.total_bits = item_end - current_pos;
                        children.push(final_child);
                        current_pos = item_end;
                        continue;
                    }
                } else { break; }
            }
        }
        current_pos += 1;
    }
    if children.is_empty() { None } else { Some((children, current_pos)) }
}
