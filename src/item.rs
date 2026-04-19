use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use std::io::{self, Cursor};
use crate::data::bit_cursor::BitCursor;

pub(crate) fn item_trace_enabled() -> bool {
    std::env::var_os("D2R_ITEM_TRACE").is_some()
}

macro_rules! item_trace {
    ($($arg:tt)*) => {
        if crate::item::item_trace_enabled() {
            eprintln!($($arg)*);
        }
    };
}

pub use crate::domain::item::{RecordedBit, ItemBitRange, BitSegment, ItemHeader, ItemBody, ItemModule, Item, ItemQuality, map_item_quality, CharmBagData, CursedItemData, BitEmitter, HuffmanTree};
pub use crate::domain::stats::{
    stat_save_bits, AlphaStatMap, ALPHA_STAT_MAPS,
    lookup_alpha_map_by_raw, lookup_alpha_map_by_effective,
    ItemProperty, ItemStats,
};
pub use crate::error::{ParsingError, ParsingFailure, ParsingResult, BackingBitCursor};
pub use crate::domain::header::entity::ItemSegmentType;

pub use crate::domain::stats::parser::{PropertyParseResult, recover_alpha_xrs_properties};

pub fn read_property_list<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)> {
    crate::domain::stats::parser::read_property_list(
        recorder,
        code,
        version,
        section_recovery,
        huffman,
        alpha_runeword,
        recover_property_reader,
    )
}

pub fn parse_single_property<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> ParsingResult<PropertyParseResult> {
    crate::domain::stats::parser::parse_single_property(
        recorder,
        code,
        version,
        section_recovery,
        huffman,
        alpha_runeword,
        recover_property_reader,
    )
}

pub use crate::engine::checksum::Checksum;

fn item_template(code: &str) -> Option<&'static crate::data::item_codes::ItemTemplate> {
    crate::data::item_codes::ITEM_TEMPLATES
        .iter()
        .find(|template| template.code == code.trim())
}

pub type PropertyReaderContext<'a> = Option<(&'a [u8], u64)>;

pub fn calculate_stat_value(raw: i32, save_add: i32) -> i32 {
    raw.wrapping_sub(save_add)
}

fn read_player_name<R: BitRead>(cursor: &mut BitCursor<R>) -> ParsingResult<String> {
    let mut name = String::new();
    loop {
        let ch = cursor.read_bits::<u8>(7)? as u8;
        if ch == 0 { break; }
        name.push(ch as char);
    }
    Ok(name)
}

fn parse_base_header<R: BitRead>(cursor: &mut BitCursor<R>, _version: u8) -> ParsingResult<(u32, u8, ItemQuality, String)> {
    let id = cursor.read_bits::<u32>(32)?;
    let level = cursor.read_bits::<u8>(7)? as u8;
    let quality_raw = cursor.read_bits::<u8>(4)? as u8;
    let quality = ItemQuality::from(quality_raw);
    Ok((id, level, quality, String::new()))
}

/// Alpha v105 compact items (potions, scrolls, basic gear) are exactly 10 bytes.
const V105_COMPACT_ITEM_BYTES: u64 = 10;

impl Item {
    fn read_item_code<R: BitRead>(
        cursor: &mut BitCursor<R>,
        is_ear: bool,
        huffman: &HuffmanTree,
        _version: u8,
    ) -> ParsingResult<(String, Option<u8>, Option<u8>, Option<String>)> {
        cursor.begin_segment(ItemSegmentType::Code);
        let mut ear_class = None;
        let mut ear_level = None;
        let mut ear_player_name = None;

        let code = if is_ear {
            let ear_class_bits = cursor.read_bits::<u8>(3)? as u8;
            let ear_level_bits = cursor.read_bits::<u8>(7)? as u8;
            let player_name = read_player_name(cursor)?;
            ear_class = Some(ear_class_bits);
            ear_level = Some(ear_level_bits);
            ear_player_name = Some(player_name);
            "ear ".to_string()
        } else {
            let mut decoded = String::new();
            for _ in 0..4 {
                decoded.push(huffman.decode_recorded(cursor)?);
            }
            decoded
        };
        cursor.end_segment();
        Ok((code, ear_class, ear_level, ear_player_name))
    }

    fn read_extended_stats<R: BitRead>(
        cursor: &mut BitCursor<R>,
        code: &str,
        is_socketed: bool,
        is_runeword: bool,
        is_personalized: bool,
        version: u8,
        alpha_mode: bool,
    ) -> ParsingResult<(
        Option<u32>,
        Option<u8>,
        Option<ItemQuality>,
        bool,
        Option<u8>,
        bool,
        Option<u16>,
        Option<u8>,
        Option<u16>,
        Option<u16>,
        Option<u8>,
        Option<u8>,
        [Option<u16>; 6],
        Option<u16>,
        Option<u16>,
        Option<u8>,
        Option<String>,
        Option<u8>,
        bool,
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<u32>,
        Option<u8>,
        u8,
    )> {
        cursor.begin_segment(ItemSegmentType::ExtendedStats);
        let trimmed_code = code.trim();
        let template = item_template(code);
        let is_alpha = alpha_mode && (version == 5 || version == 1);

        let (item_id, item_level, item_quality, has_multiple_graphics, has_class_specific_data, _timestamp_flag) = if is_alpha {
            let level = cursor.read_bits::<u8>(7)? as u8;
            let _alpha_mid_pad = cursor.read_bit()?; // 1 bit unknown between level/quality
            let quality_raw = cursor.read_bits::<u8>(3)? as u8;
            let quality = ItemQuality::from(quality_raw);
            
            let has_multiple_graphics = cursor.read_bit()?;
            let has_class_specific_data = cursor.read_bit()?;
            let timestamp_flag = cursor.read_bit()?;
            
            item_trace!("[DEBUG v5] Lvl={}, Qual={:?}, multi_gfx={}, class_data={}, timestamp={}", level, quality, has_multiple_graphics, has_class_specific_data, timestamp_flag);
            
            (Some(0u32), Some(level), Some(quality), has_multiple_graphics, has_class_specific_data, timestamp_flag)
        } else {
            let (id, level, quality, _code) = parse_base_header(cursor, version)?;
            (Some(id), Some(level), Some(quality), false, false, false)
        };

        let mut multi_graphics_bits = None;
        let mut class_specific_bits = None;

        if is_alpha {
            if has_multiple_graphics {
                multi_graphics_bits = Some(cursor.read_bits::<u8>(3)? as u8);
            }
            if has_class_specific_data {
                class_specific_bits = Some(cursor.read_bits::<u16>(3)? as u16);
            }
        } else {
            if has_multiple_graphics {
                multi_graphics_bits = Some(cursor.read_bits::<u8>(3)? as u8);
            }
            if has_class_specific_data {
                class_specific_bits = Some(cursor.read_bits::<u16>(11)? as u16);
            }
        }
        
        let quality_val = item_quality.unwrap_or(ItemQuality::Normal);

        let mut low_high_graphic_bits = None;
        let mut magic_prefix = None;
        let mut magic_suffix = None;
        let mut rare_name_1 = None;
        let mut rare_name_2 = None;
        let mut rare_affixes = [None; 6];
        let mut unique_id = None;

        match quality_val {
            ItemQuality::Low | ItemQuality::High => {
                low_high_graphic_bits = Some(cursor.read_bits::<u8>(3)? as u8);
            }
            ItemQuality::Magic => {
                if is_alpha {
                    let pre = cursor.read_bits::<u16>(7)? as u16;
                    let suf = cursor.read_bits::<u16>(7)? as u16;
                    magic_prefix = Some(pre);
                    magic_suffix = Some(suf);
                } else {
                    magic_prefix = Some(cursor.read_bits::<u16>(11)? as u16);
                    magic_suffix = Some(cursor.read_bits::<u16>(11)? as u16);
                }
            }
            ItemQuality::Rare | ItemQuality::Crafted => {
                rare_name_1 = Some(cursor.read_bits::<u8>(8)? as u8);
                rare_name_2 = Some(cursor.read_bits::<u8>(8)? as u8);
                for i in 0..6 {
                    if cursor.read_bit()? {
                        rare_affixes[i] = Some(cursor.read_bits::<u16>(11)? as u16);
                    }
                }
            }
            ItemQuality::Set | ItemQuality::Unique => {
                unique_id = Some(cursor.read_bits::<u16>(12)? as u16);
            }
            _ => {}
        }

        let mut runeword_id = None;
        let mut runeword_level = None;
        if is_runeword && !is_alpha {
            let id = cursor.read_bits::<u16>(12)? as u16;
            runeword_id = Some(id);
            runeword_level = Some(cursor.read_bits::<u8>(4)? as u8);
        }

        let mut personalized_player_name = None;
        if is_personalized {
            personalized_player_name = Some(read_player_name(cursor)?);
        }

        let tbk_ibk_teleport = if trimmed_code == "tbk" || trimmed_code == "ibk" {
            Some(cursor.read_bits::<u8>(5)? as u8)
        } else {
            None
        };

        let timestamp_flag = if is_alpha {
            _timestamp_flag
        } else {
            cursor.read_bits::<u8>(1)? != 0
        };

        let (reads_defense, reads_durability, reads_quantity) = if let Some(template) = template {
            (template.is_armor, template.has_durability, template.is_stackable)
        } else {
            let armor_like_unknown = has_class_specific_data || trimmed_code.contains(' ');
            (armor_like_unknown, armor_like_unknown, false)
        };

        let (mut defense, mut max_durability, mut current_durability, mut quantity, mut sockets) = (None, None, None, None, None);

        if is_alpha {
            if reads_defense {
                defense = Some(cursor.read_bits::<u32>(11)?);
            }
            if reads_durability {
                let max_dur_bits = stat_save_bits(73).unwrap_or(8);
                let cur_bits = stat_save_bits(72).unwrap_or(9);
                let m_dur = cursor.read_bits::<u32>(max_dur_bits)?;
                max_durability = Some(m_dur);
                if m_dur > 0 {
                    current_durability = Some(cursor.read_bits::<u32>(cur_bits)?);
                    if version != 5 {
                        let _extra = cursor.read_bit()?;
                    }
                }
            }
            if reads_quantity {
                quantity = Some(cursor.read_bits::<u32>(9)?);
            }
            if is_socketed {
                sockets = Some(cursor.read_bits::<u8>(4)? as u8);
            }
            
            let mut set_list_count = 0;
            if item_quality == Some(ItemQuality::Set) {
                let set_list_value = cursor.read_bits::<u8>(5)?;
                set_list_count = match set_list_value {
                    1 | 2 | 4 => 1,
                    3 | 6 | 10 | 12 => 2,
                    7 => 3,
                    15 => 4,
                    31 => 5,
                    _ => 0,
                };
            }

            cursor.end_segment();
            return Ok((
                item_id, item_level, item_quality,
                has_multiple_graphics, multi_graphics_bits,
                has_class_specific_data, class_specific_bits,
                low_high_graphic_bits, magic_prefix, magic_suffix,
                rare_name_1, rare_name_2, rare_affixes,
                unique_id, runeword_id, runeword_level,
                personalized_player_name, tbk_ibk_teleport,
                timestamp_flag, defense, max_durability, current_durability, quantity, sockets, set_list_count
            ));
        }

        if reads_defense {
            let defense_bits = stat_save_bits(31).unwrap_or(11);
            defense = Some(cursor.read_bits::<u32>(defense_bits)?);
        }

        if reads_durability {
            let max_dur_bits = stat_save_bits(73).unwrap_or(8);
            let cur_bits = stat_save_bits(72).unwrap_or(9);
            let m_dur = cursor.read_bits::<u32>(max_dur_bits)?;
            max_durability = Some(m_dur);
            if m_dur > 0 {
                current_durability = Some(cursor.read_bits::<u32>(cur_bits)?);
                let _dur_extra = cursor.read_bit()?;
            }
        }

        if reads_quantity {
            quantity = Some(cursor.read_bits::<u32>(9)?);
        }

        if is_socketed {
            sockets = Some(cursor.read_bits::<u8>(4)? as u8);
        }

        let mut set_list_count = 0;
        if item_quality == Some(ItemQuality::Set) {
            let set_list_value = cursor.read_bits::<u8>(5)?;
            set_list_count = match set_list_value {
                1 | 2 | 4 => 1,
                3 | 6 | 10 | 12 => 2,
                7 => 3,
                15 => 4,
                31 => 5,
                _ => 0,
            };
        }

        cursor.end_segment();
        Ok((
            item_id, item_level, item_quality,
            has_multiple_graphics, multi_graphics_bits,
            has_class_specific_data, class_specific_bits,
            low_high_graphic_bits, magic_prefix, magic_suffix,
            rare_name_1, rare_name_2, rare_affixes,
            unique_id, runeword_id, runeword_level,
            personalized_player_name, tbk_ibk_teleport,
            timestamp_flag, defense, max_durability, current_durability, quantity, sockets, set_list_count,
        ))
    }

    fn read_item_stats<R: BitRead>(
        cursor: &mut BitCursor<R>,
        code: &str,
        version: u8,
        quality: Option<ItemQuality>,
        set_list_count: u8,
        is_runeword: bool,
        _is_personalized: bool,
        ctx: Option<(&[u8], u64)>,
        huffman: &HuffmanTree,
        alpha_mode: bool,
    ) -> ParsingResult<(Vec<ItemProperty>, Vec<Vec<ItemProperty>>, Vec<ItemProperty>, bool, bool)> {
        cursor.begin_segment(ItemSegmentType::Stats);
        let trimmed_code = code.trim();
        let is_alpha = alpha_mode && (version == 5 || version == 1);
        let quality_val = quality.unwrap_or(ItemQuality::Normal);

        let (mut properties, mut properties_complete, terminator_bit): (Vec<ItemProperty>, bool, bool) = if is_alpha && quality_val == ItemQuality::Normal && !is_runeword && !trimmed_code.is_empty() {
             // In Alpha v105, Normal items are truly compact and skip the property list entirely.
             (Vec::new(), true, false)
        } else {
            read_property_list(cursor, trimmed_code, version, ctx, huffman, false)?
        };

        if is_alpha && is_runeword && trimmed_code == "xrs" && properties.is_empty() {
            if let Some((section_bytes, item_start_bit)) = ctx {
                let recovered = recover_alpha_xrs_properties(section_bytes, item_start_bit);
                if !recovered.is_empty() {
                    properties = recovered;
                    properties_complete = true;
                }
            }
        }

        let mut set_attributes = Vec::new();
        let mut runeword_attributes = Vec::new();

        let mut parse_property_lists = properties_complete;
        if parse_property_lists && quality == Some(ItemQuality::Set) && set_list_count > 0 {
            for _ in 0..set_list_count {
                let (set_props, complete, _term_bit) =
                    read_property_list(cursor, trimmed_code, version, ctx, huffman, false)?;
                set_attributes.push(set_props);
                if !complete {
                    parse_property_lists = false;
                    break;
                }
            }
        }

        if parse_property_lists && is_runeword {
            let (rw_props, _complete, _term_bit) =
                read_property_list(cursor, trimmed_code, version, ctx, huffman, true)?;
            runeword_attributes = rw_props;
        }

        cursor.end_segment();
        Ok((properties, set_attributes, runeword_attributes, properties_complete, terminator_bit))
    }

    pub fn from_reader_with_context<R: BitRead>(
        cursor: &mut BitCursor<R>,
        huffman: &HuffmanTree,
        ctx: Option<(&[u8], u64)>,
        alpha_mode: bool,
    ) -> ParsingResult<Item> {
        cursor.set_trace(item_trace_enabled());
        let start_bit = cursor.pos();
        cursor.begin_segment(ItemSegmentType::Root);
        
        let axiom = crate::domain::header::entity::HeaderAxiom {
            version: 0, 
            alpha_mode,
        };

        let header = crate::domain::header::parser::parse_header(cursor, &axiom, huffman, ctx)?;
        
        let flags = header.flags;
        let version = header.version;
        let mode = header.mode;
        let loc = header.location;
        let x = header.x;
        let y = header.y;
        let page = header.page;
        let header_socket_hint = header.socket_hint;
        let is_compact = header.is_compact;
        let is_ear = header.is_ear;
        let is_identified = header.is_identified;
        let is_personalized = header.is_personalized;
        let is_runeword = header.is_runeword;
        let is_ethereal = header.is_ethereal;
        let is_socketed = header.is_socketed;

        let (code, ear_class, ear_level, ear_player_name) = Self::read_item_code(cursor, is_ear, huffman, version)?;

        if is_ear {
            let end_bit = cursor.pos();
            let item = Item {
                bits: cursor.recorded_bits().to_vec(),
                code, flags, version, is_ear, ear_class, ear_level, ear_player_name,
                personalized_player_name: None, mode, x, y, page, location: loc,
                header_socket_hint, has_multiple_graphics: false, multi_graphics_bits: None,
                has_class_specific_data: false, class_specific_bits: None, id: None, level: None, quality: None,
                low_high_graphic_bits: None, is_compact: false, is_socketed: false, is_identified,
                is_personalized, is_runeword: false, is_ethereal,
                magic_prefix: None, magic_suffix: None, rare_name_1: None, rare_name_2: None,
                rare_affixes: [None; 6], unique_id: None, runeword_id: None, runeword_level: None,
                properties: Vec::new(), set_attributes: Vec::new(), runeword_attributes: Vec::new(),
                num_socketed_items: 0, socketed_items: Vec::new(), timestamp_flag: false, properties_complete: false,
                set_list_count: 0, tbk_ibk_teleport: None, defense: None, max_durability: None,
                current_durability: None, quantity: None, sockets: None, modules: Vec::new(),
                range: ItemBitRange { start: start_bit, end: end_bit }, total_bits: 0, gap_bits: Vec::new(),
                terminator_bit: false, segments: cursor.segments().to_vec(),
            };
            cursor.end_segment();
            return Ok(item);
        }

        let stats = if !is_compact {
            Self::read_extended_stats(cursor, &code, is_socketed, is_runeword, is_personalized, version, alpha_mode)?
        } else {
            (None, None, None, false, None, false, None, None, None, None, None, None, [None; 6], None, None, None, None, None, false, None, None, None, None, None, 0)
        };

        let item_id = stats.0;
        let item_level = stats.1;
        let item_quality = stats.2;
        cursor.alpha_quality = item_quality;
        let has_multiple_graphics = stats.3;
        let multi_graphics_bits = stats.4;
        let has_class_specific_data = stats.5;
        let class_specific_bits = stats.6;
        let low_high_graphic_bits = stats.7;
        let magic_prefix = stats.8;
        let magic_suffix = stats.9;
        let rare_name_1 = stats.10;
        let rare_name_2 = stats.11;
        let rare_affixes = stats.12;
        let unique_id = stats.13;
        let runeword_id = stats.14;
        let runeword_level = stats.15;
        let personalized_player_name = stats.16;
        let tbk_ibk_teleport = stats.17;
        let timestamp_flag = stats.18;
        let defense = stats.19;
        let max_durability = stats.20;
        let current_durability = stats.21;
        let quantity = stats.22;
        let sockets = stats.23;
        let set_list_count = stats.24;

        let (properties, set_attributes, runeword_attributes, properties_complete, terminator_bit) = if !is_compact {
            Self::read_item_stats(cursor, &code, version, item_quality, set_list_count, is_runeword, is_personalized, ctx, huffman, alpha_mode)?
        } else {
            (Vec::new(), Vec::new(), Vec::new(), true, false)
        };

        let end_bit = cursor.pos();

        let item = Item {
            bits: cursor.recorded_bits().to_vec(),
            code, flags, version, is_ear, ear_class, ear_level, ear_player_name, personalized_player_name,
            mode, x, y, page, location: loc, header_socket_hint, has_multiple_graphics, multi_graphics_bits,
            has_class_specific_data, class_specific_bits, id: item_id, level: item_level, quality: item_quality,
            low_high_graphic_bits, is_compact, is_socketed, is_identified, is_personalized, is_runeword,
            is_ethereal, magic_prefix, magic_suffix, rare_name_1, rare_name_2, rare_affixes, unique_id,
            runeword_id, runeword_level, properties, set_attributes, runeword_attributes,
            num_socketed_items: header_socket_hint, socketed_items: Vec::new(), timestamp_flag,
            properties_complete, terminator_bit, set_list_count, tbk_ibk_teleport, defense,
            max_durability, current_durability, quantity, sockets, modules: Vec::new(),
            range: ItemBitRange { start: start_bit, end: end_bit }, total_bits: 0, gap_bits: Vec::new(),
            segments: cursor.segments().to_vec(),
        };
        cursor.end_segment();
        Ok(item)
    }

    pub fn from_reader<R: BitRead>(reader: &mut R, huffman: &HuffmanTree, alpha_mode: bool) -> ParsingResult<Self> {
        let mut cursor = BitCursor::new(reader);
        Self::from_reader_with_context(&mut cursor, huffman, None, alpha_mode)
    }

    pub fn from_bytes(bytes: &[u8], huffman: &HuffmanTree, alpha_mode: bool) -> ParsingResult<Self> {
        let mut reader = IoBitReader::endian(io::Cursor::new(bytes), LittleEndian);
        let mut cursor = BitCursor::new(reader);
        Self::from_reader_with_context(&mut cursor, huffman, Some((bytes, 0)), alpha_mode)
    }

    pub fn read_player_items(bytes: &[u8], huffman: &HuffmanTree, alpha_mode: bool) -> ParsingResult<Vec<Item>> {
        let jm_pos = (0..bytes.len().saturating_sub(1))
            .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
            .ok_or_else(|| ParsingFailure {
                error: ParsingError::MissingMarker { marker: "JM".to_string(), bit_offset: 0 },
                context_stack: vec!["read_player_items".to_string()],
                bit_offset: 0, context_relative_offset: 0, hint: None,
            })?;
        let top_level_count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
        // For Alpha v105, we don't prematurely cut the section at the next "JM" marker, 
        // as nested items also use "JM" and we rely on the top_level_count.
        Self::read_section(&bytes[jm_pos + 4..], top_level_count, huffman, alpha_mode)
    }

    pub fn read_section(
        section_bytes: &[u8],
        top_level_count: u16,
        huffman: &HuffmanTree,
        alpha_mode: bool,
    ) -> ParsingResult<Vec<Item>> {
        let section_bits = (section_bytes.len() * 8) as u64;
        let mut items: Vec<Item> = Vec::with_capacity(top_level_count as usize);
        let mut bit_pos = 0u64;

        while bit_pos < section_bits && items.len() < top_level_count as usize {
            let start = find_next_item_match(section_bytes, bit_pos, huffman, alpha_mode).unwrap_or(section_bits);
            if start >= section_bits { break; }

            let (item, consumed_bits) = match parse_item_at(section_bytes, start, huffman, items.len(), alpha_mode) {
                Ok(res) => res,
                Err(e) => { 
                    item_trace!("[DEBUG] Failed to parse item at {}: {:?}", start, e);
                    bit_pos = start + 1; continue; 
                }
            };
            
            item_trace!("[DEBUG] Item {} parsed. Code: {}, Offset: {}, Consumed: {}", items.len(), item.code, start, consumed_bits);

            let mut end = start + consumed_bits;
            
            // Alpha v105 Lookahead Rescue: 
            // If we are in Alpha mode, try to find the NEXT item's real start 
            // to bound the current item and prevent overrun.
            if alpha_mode && items.len() < (top_level_count as usize - 1) {
                if let Some(next_real_start) = find_next_item_match(section_bytes, start + 72, huffman, alpha_mode) {
                    if next_real_start < end {
                        item_trace!("[DEBUG] Alpha Rescue: Trimming item {} end from {} to {}", items.len(), end, next_real_start);
                        end = next_real_start;
                    }
                }
            } else if alpha_mode && item.is_compact {
                // Compact items are exactly 10 bytes (80 bits) in Alpha v105 if no other item follows immediately.
                let standard_end = start + V105_COMPACT_ITEM_BYTES * 8;
                if end < standard_end {
                    end = standard_end;
                }
            }
            
            bit_pos = end;
            let mut final_item = item;
            let gap_bits = Vec::new();
            final_item.range.start = start;
            final_item.range.end = end;
            final_item.total_bits = end - start;
            final_item.gap_bits = gap_bits;

            if alpha_mode {
                final_item.bits.clear();
                let mut reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
                if reader.skip(start as u32).is_ok() {
                    for i in 0..(end - start) {
                        if let Ok(bit) = reader.read_bit() {
                            final_item.bits.push(RecordedBit { bit, offset: start + i as u64 });
                        }
                    }
                }
            }

            if final_item.mode == 6 || final_item.location == 6 {
                if let Some(parent) = items.last_mut() { parent.socketed_items.push(final_item); }
            } else {
                items.push(final_item);
                let is_last_top_level = items.len() == top_level_count as usize;
                let parent_index = items.len().saturating_sub(1);
                if let Some(parent) = items.last_mut() {
                    if parent.is_socketed || parent.is_runeword || is_last_top_level || alpha_mode {
                        let rescue_limit = socket_rescue_limit(parent);
                        if let Some((rescued_children, rescued_end)) = scan_socket_children(section_bytes, bit_pos, huffman, parent_index, alpha_mode, rescue_limit) {
                            parent.socketed_items.extend(rescued_children);
                            bit_pos = rescued_end;
                        }
                    }
                }
            }
        }
        Ok(items)
    }

    pub fn scan_items(bytes: &[u8], huffman: &HuffmanTree) -> ParsingResult<Vec<(u64, String)>> {
        let mut results = Vec::new();
        let limit = bytes.len() as u64 * 8;
        for bit in 0..(limit.saturating_sub(100)) {
            let mut reader = IoBitReader::endian(Cursor::new(bytes), LittleEndian);
            let _ = reader.skip(bit as u32);
            let mut cursor = BitCursor::new(reader);
            if let Ok(item) = Item::from_reader_with_context(&mut cursor, huffman, Some((bytes, bit)), false) {
                if cursor.pos() >= 32 {
                    results.push((bit, item.code.clone()));
                }
            }
        }
        Ok(results)
    }

    pub fn empty_for_tests() -> Self {
        Item {
            bits: Vec::new(), code: "    ".to_string(), flags: 0, version: 0, is_ear: false,
            ear_class: None, ear_level: None, ear_player_name: None, personalized_player_name: None,
            mode: 0, x: 0, y: 0, page: 0, location: 0, header_socket_hint: 0, has_multiple_graphics: false,
            multi_graphics_bits: None, has_class_specific_data: false, class_specific_bits: None,
            id: None, level: None, quality: None, low_high_graphic_bits: None, is_compact: false,
            is_socketed: false, is_identified: false, is_personalized: false, is_runeword: false,
            is_ethereal: false, magic_prefix: None, magic_suffix: None, rare_name_1: None, rare_name_2: None,
            rare_affixes: [None; 6], unique_id: None, runeword_id: None, runeword_level: None,
            properties: Vec::new(), set_attributes: Vec::new(), runeword_attributes: Vec::new(),
            num_socketed_items: 0, socketed_items: Vec::new(), timestamp_flag: false, properties_complete: false,
            set_list_count: 0, tbk_ibk_teleport: None, defense: None, max_durability: None,
            current_durability: None, quantity: None, sockets: None, modules: Vec::new(),
            range: ItemBitRange { start: 0, end: 0 }, total_bits: 0, gap_bits: Vec::new(),
            terminator_bit: false, segments: Vec::new(),
        }
    }
}

pub fn peek_item_header_at(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
    alpha_mode: bool,
) -> Option<(u8, u8, u8, String, u32, u8, bool, u64, i8)> {
    let mut reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
    if reader.skip(start_bit as u32).is_err() { return None; }
    let mut cursor = BitCursor::new(reader);

    let flags = cursor.read_bits::<u32>(32).ok()?;
    let version = cursor.read_bits::<u32>(3).ok()? as u8;
    let mode = cursor.read_bits::<u32>(3).ok()? as u8;
    let loc = cursor.read_bits::<u32>(3).ok()? as u8;
    let x = cursor.read_bits::<u32>(4).ok()? as u8;
    let is_compact = (flags & (1 << 21)) != 0;

    let mut header_len = 45;
    if version == 5 {
        header_len = 53; // Alpha headers are always 53 bits (45 + 8 padding)
    } else if !is_compact {
        // y (4), page (3), socket_hint (3)
        let _ = cursor.read_bits::<u32>(10).ok()?;
        header_len += 10;
    }

    if !alpha_mode {
        let mut code = String::new();
        let mut n_reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
        let n_start = start_bit + header_len as u64;
        if n_reader.skip(n_start as u32).is_err() { return None; }
        let mut n_cursor = BitCursor::new(n_reader);
        for _ in 0..4 {
            if let Ok(ch) = huffman.decode_recorded(&mut n_cursor) {
                code.push(ch);
            } else { return None; }
        }
        // Return bits read to code start
        return Some((mode, loc, x, code, flags, version, is_compact, header_len as u64, 0));
    } else {
        // Alpha v105 nudge search: Byte-aligned starts are the norm after Slice 6.2/6.3.
        // +8 and -1 are legacy recovery fallbacks for bit-drifted starts.
        let nudges: [i32; 3] = [0, 8, -1];
        let mut best_candidate: Option<(u8, u8, u8, String, u32, u8, bool, u64, i32)> = None;

        for &nudge in nudges.iter() {
            let mut n_reader = IoBitReader::endian(Cursor::new(section_bytes), LittleEndian);
            let n_start = (start_bit as i64 + header_len as i64 + nudge as i64) as u64;
            if n_start >= (section_bytes.len() * 8) as u64 { continue; }
            if n_reader.skip(n_start as u32).is_err() { continue; }
            let mut n_cursor = BitCursor::new(n_reader);
            let mut code = String::new();
            let mut ok = true;
            for _ in 0..4 {
                if let Ok(ch) = huffman.decode_recorded(&mut n_cursor) {
                    code.push(ch);
                } else { ok = false; break; }
            }
            if ok && is_plausible_item_header(mode, loc, &code, flags, version, true) {
                let candidate = (mode, loc, x, code.clone(), flags, version, is_compact, header_len as u64, nudge);
                
                item_trace!("[DEBUG v5] Candidate Alpha header: code='{}', nudge={}, compact={}, bits={}", 
                    code, nudge, is_compact, start_bit as i64 + header_len as i64 + nudge as i64);

                // Selection logic: Nudge 0 is gold. Lower absolute nudge is better.
                if nudge == 0 {
                    return Some((mode, loc, x, code, flags, version, is_compact, header_len as u64, 0));
                }
                
                if best_candidate.is_none() || nudge.abs() < best_candidate.as_ref().unwrap().8.abs() {
                    best_candidate = Some(candidate);
                }
            }
        }
        return best_candidate.map(|(m, l, x, c, f, v, ic, hl, n)| (m, l, x, c, f, v, ic, hl, n as i8));
    }
}

pub fn recover_property_reader<R: BitRead>(
    cursor: &mut BitCursor<R>,
    _code: &str,
    section_bytes: &[u8],
    item_start_bit: u64,
    huffman: &HuffmanTree,
) -> ParsingResult<bool> {
    let section_bits = (section_bytes.len() * 8) as u64;
    let section_pos = item_start_bit + cursor.pos();
    let mut probe = section_pos;
    while probe < section_bits {
        if let Some((mode, location, _x, p_code, p_flags, p_version, _is_c, _h_bits, _nudge)) = peek_item_header_at(section_bytes, probe, huffman, true) {
            if is_plausible_item_header(mode, location, &p_code, p_flags, p_version, true) {
                let skip = if probe > section_pos { probe - section_pos } else { 0 };
                for _ in 0..skip { cursor.read_bit()?; }
                return Ok(true);
            }
        }
        probe += 1;
    }
    Ok(false)
}

pub fn is_plausible_item_header(
    mode: u8,
    location: u8,
    code: &str,
    flags: u32,
    version: u8,
    alpha_mode: bool,
) -> bool {
    if code.len() < 3 { return false; }
    if !code.chars().all(|c| c.is_alphanumeric() || c == ' ') { return false; }
    
    // Strict template check for Alpha v105 to avoid Huffman "w" drift
    if alpha_mode && item_template(code).is_none() {
        return false;
    }

    if alpha_mode {
        if version != 5 && version != 1 { return false; }
        if mode > 6 || location > 15 { return false; }
        if (flags & 0xF8000000) != 0 { return false; }
        
        // Alpha v105 sanity
        let _is_compact = (flags & (1 << 21)) != 0;
        // In Alpha v105, some items (Javelins/Bucklers) might be non-compact even if normal.
        // We rely on Huffman code match and Nudge preference instead of strict flag predicates.
    } else {
        if mode > 6 || location > 15 { return false; }
    }
    true
}

fn socket_rescue_limit(_parent: &Item) -> u64 { 256 }

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
                    if let Ok((item, consumed)) = parse_item_at(bytes, current_pos, huffman, 0, alpha) {
                        let mut item_end = current_pos + consumed;
                        
                        // Alpha v105 Lookahead Rescue for nested items
                        if alpha {
                            // Probe for next child or next top-level item
                            if let Some(next_start) = find_next_item_match(bytes, current_pos + 72, huffman, alpha) {
                                if next_start < item_end && next_start < max_pos {
                                    item_trace!("[DEBUG] Alpha Child Rescue: Trimming child end from {} to {}", item_end, next_start);
                                    item_end = next_start;
                                }
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
                } else {
                    // Reached next top-level item
                    break;
                }
            }
        }
        current_pos += 1;
    }

    if children.is_empty() { None } else { Some((children, current_pos)) }
}

fn find_next_item_match(bytes: &[u8], pos: u64, huffman: &HuffmanTree, alpha: bool) -> Option<u64> {
    let limit = (bytes.len() * 8) as u64;
    let mut probe = pos;
    while probe < limit {
        // 1-bit granular probing for Alpha v105
        if let Some((mode, location, _x, code, flags, version, _is_compact, _header_bits, _nudge)) = peek_item_header_at(bytes, probe, huffman, alpha) {
            if is_plausible_item_header(mode, location, &code, flags, version, alpha) {
                return Some(probe);
            }
        }
        probe += 1;
    }
    None
}

fn parse_item_at(
    bytes: &[u8],
    bit: u64,
    huff: &HuffmanTree,
    _idx: usize,
    alpha: bool,
) -> ParsingResult<(Item, u64)> {
    let mut reader = IoBitReader::endian(Cursor::new(bytes), LittleEndian);
    let _ = reader.skip(bit as u32);
    let mut cursor = BitCursor::new(reader);
    let item = Item::from_reader_with_context(&mut cursor, huff, Some((bytes, bit)), alpha)?;
    Ok((item, cursor.pos()))
}
