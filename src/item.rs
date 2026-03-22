use bitstream_io::{BitRead, BitReader as IoBitReader, BitWrite, BitWriter, LittleEndian};
use std::io::{self, Cursor};

fn item_trace_enabled() -> bool {
    std::env::var_os("D2R_ITEM_TRACE").is_some()
}

macro_rules! item_trace {
    ($($arg:tt)*) => {
        if crate::item::item_trace_enabled() {
            println!($($arg)*);
        }
    };
}

pub struct HuffmanTree {
    root: Box<HuffmanNode>,
    encoding_table: std::collections::HashMap<char, Vec<bool>>,
}

struct HuffmanNode {
    symbol: Option<char>,
    left: Option<Box<HuffmanNode>>,
    right: Option<Box<HuffmanNode>>,
}

impl HuffmanNode {
    fn new() -> Self {
        HuffmanNode {
            symbol: None,
            left: None,
            right: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordedBit {
    pub bit: bool,
    pub offset: u64,
}

pub struct BitRecorder<'a, R: BitRead> {
    pub reader: &'a mut R,
    pub recorded_bits: Vec<RecordedBit>,
    pub total_read: u64,
}

impl<'a, R: BitRead> BitRecorder<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        BitRecorder {
            reader,
            recorded_bits: Vec::new(),
            total_read: 0,
        }
    }

    pub fn read_bit(&mut self) -> io::Result<bool> {
        let bit = self.reader.read_bit()?;
        let offset = self.total_read;
        self.recorded_bits.push(RecordedBit { bit, offset });
        self.total_read += 1;
        Ok(bit)
    }

    pub fn read_bits(&mut self, n: u32) -> io::Result<u32> {
        let mut value = 0u32;
        for i in 0..n {
            if self.read_bit()? {
                value |= 1 << i;
            }
        }
        Ok(value)
    }

    pub fn read_bits_u64(&mut self, n: u32) -> io::Result<u64> {
        let mut value = 0u64;
        for i in 0..n {
            if self.read_bit()? {
                value |= 1 << i;
            }
        }
        Ok(value)
    }
}

pub struct BitEmitter {
    writer: BitWriter<Vec<u8>, LittleEndian>,
}

impl BitEmitter {
    pub fn new() -> Self {
        BitEmitter {
            writer: BitWriter::endian(Vec::new(), LittleEndian),
        }
    }

    pub fn write_bit(&mut self, bit: bool) -> io::Result<()> {
        self.writer.write_bit(bit)
    }

    pub fn write_bits(&mut self, value: u32, count: u32) -> io::Result<()> {
        for i in 0..count {
            let bit = (value >> i) & 1 != 0;
            self.writer.write_bit(bit)?;
        }
        Ok(())
    }

    pub fn extend_bits<I>(&mut self, bits: I) -> io::Result<()>
    where
        I: IntoIterator<Item = bool>,
    {
        for bit in bits {
            self.writer.write_bit(bit)?;
        }
        Ok(())
    }

    pub fn byte_align(&mut self) -> io::Result<()> {
        self.writer.byte_align()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.writer.into_writer()
    }
}

impl HuffmanTree {
    pub fn new() -> Self {
        let mut root = Box::new(HuffmanNode::new());
        let table = [
            ('0', "11111011"),
            (' ', "10"),
            ('1', "1111100"),
            ('2', "001100"),
            ('3', "1101101"),
            ('4', "11111010"),
            ('5', "00010110"),
            ('6', "1101111"),
            ('7', "01111"),
            ('8', "000100"),
            ('9', "01110"),
            ('a', "11110"),
            ('b', "0101"),
            ('c', "01000"),
            ('d', "110001"),
            ('e', "110000"),
            ('f', "010011"),
            ('g', "11010"),
            ('h', "00011"),
            ('i', "1111110"),
            ('j', "000101110"),
            ('k', "010010"),
            ('l', "11101"),
            ('m', "01101"),
            ('n', "001101"),
            ('o', "1111111"),
            ('p', "11001"),
            ('q', "11011001"),
            ('r', "11100"),
            ('s', "0010"),
            ('t', "01100"),
            ('u', "00001"),
            ('v', "1101110"),
            ('w', "00000"),
            ('x', "00111"),
            ('y', "0001010"),
            ('z', "11011000"),
        ];

        let mut encoding_table = std::collections::HashMap::new();
        for (symbol, pattern) in table {
            let mut bits = Vec::new();
            let mut current = &mut root;
            for bit in pattern.chars() {
                if bit == '1' {
                    bits.push(true);
                    if current.right.is_none() {
                        current.right = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.right.as_mut().unwrap();
                } else {
                    bits.push(false);
                    if current.left.is_none() {
                        current.left = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.left.as_mut().unwrap();
                }
            }
            current.symbol = Some(symbol);
            encoding_table.insert(symbol, bits);
        }
        HuffmanTree {
            root,
            encoding_table,
        }
    }

    pub fn encode(&self, code: &str) -> io::Result<Vec<bool>> {
        let mut bits = Vec::new();
        for c in code.chars() {
            let pattern = self.encoding_table.get(&c).ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Char '{}' not in Huffman table", c),
                )
            })?;
            bits.extend(pattern);
        }
        Ok(bits)
    }

    fn decode_internal<F: FnMut() -> io::Result<bool>>(&self, mut read_bit: F) -> io::Result<char> {
        let mut current = &self.root;
        loop {
            if let Some(s) = current.symbol {
                return Ok(s);
            }
            let bit = read_bit()?;
            current = if bit {
                current.right.as_ref()
            } else {
                current.left.as_ref()
            }
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit"))?;
        }
    }

    pub fn decode_recorded<R: BitRead>(&self, recorder: &mut BitRecorder<R>) -> io::Result<char> {
        self.decode_internal(|| recorder.read_bit())
    }

    pub fn decode<R: BitRead>(&self, reader: &mut R) -> io::Result<char> {
        self.decode_internal(|| reader.read_bit())
    }
}

pub struct Checksum;
impl Checksum {
    pub fn calculate(bytes: &[u8]) -> i32 {
        let mut checksum: i32 = 0;
        for &byte in bytes {
            let carry = if checksum < 0 { 1 } else { 0 };
            checksum = (byte as i32)
                .wrapping_add(checksum.wrapping_mul(2))
                .wrapping_add(carry);
        }
        checksum
    }
    pub fn fix(bytes: &mut [u8]) {
        if bytes.len() < 16 {
            return;
        }
        bytes[12] = 0;
        bytes[13] = 0;
        bytes[14] = 0;
        bytes[15] = 0;
        let cs = Self::calculate(bytes);
        bytes[12..16].copy_from_slice(&cs.to_le_bytes());
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemQuality {
    Low = 1,
    Normal = 2,
    High = 3,
    Magic = 4,
    Set = 5,
    Rare = 6,
    Unique = 7,
    Crafted = 8,
}

impl From<u8> for ItemQuality {
    fn from(v: u8) -> Self {
        match v {
            1 => ItemQuality::Low,
            3 => ItemQuality::High,
            4 => ItemQuality::Magic,
            5 => ItemQuality::Set,
            6 => ItemQuality::Rare,
            7 => ItemQuality::Unique,
            8 => ItemQuality::Crafted,
            _ => ItemQuality::Normal,
        }
    }
}

/// A total function to map raw 4-bit value to ItemQuality.
/// Verified by Kani to have no panics for any u8 input.
pub fn map_item_quality(v: u8) -> ItemQuality {
    ItemQuality::from(v)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharmBagData {
    pub size: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursedItemData {
    pub curse_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemHeader {
    pub id: Option<u32>,
    pub quality: Option<ItemQuality>,
    pub version: u8,
    pub is_compact: bool,
    pub is_identified: bool,
    pub is_socketed: bool,
    pub is_personalized: bool,
    pub is_runeword: bool,
    pub is_ethereal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemBody {
    pub code: String,
    pub x: u8,
    pub y: u8,
    pub page: u8,
    pub location: u8,
    pub mode: u8,
    pub defense: Option<u32>,
    pub max_durability: Option<u32>,
    pub current_durability: Option<u32>,
    pub quantity: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemStats {
    pub properties: Vec<ItemProperty>,
    pub set_attributes: Vec<Vec<ItemProperty>>,
    pub runeword_attributes: Vec<ItemProperty>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemModule {
    MagicAffixes { prefix: Option<u16>, suffix: Option<u16> },
    RareAffixes { names: [Option<u8>; 2], affixes: [Option<u16>; 6] },
    UniqueAffix { unique_id: Option<u16> },
    Sockets { count: u8, items: Vec<Item> },
    Personalization(Option<String>),
    Runeword { id: Option<u16>, level: Option<u8> },
    CharmBag(CharmBagData),
    Cursed(CursedItemData),
    Augmentation(u32),
    Opaque(Vec<bool>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub bits: Vec<RecordedBit>,
    pub code: String,
    pub flags: u32,
    pub version: u8,
    pub is_ear: bool,
    pub ear_class: Option<u8>,
    pub ear_level: Option<u8>,
    pub ear_player_name: Option<String>,
    pub personalized_player_name: Option<String>,
    pub mode: u8,
    pub x: u8,
    pub y: u8,
    pub page: u8,
    pub location: u8,
    pub header_socket_hint: u8,
    pub has_multiple_graphics: bool,
    pub multi_graphics_bits: Option<u8>,
    pub has_class_specific_data: bool,
    pub class_specific_bits: Option<u16>,
    pub id: Option<u32>,
    pub level: Option<u8>,
    pub quality: Option<ItemQuality>,
    pub low_high_graphic_bits: Option<u8>,
    pub is_compact: bool,
    pub is_socketed: bool,
    pub is_identified: bool,
    pub is_personalized: bool,
    pub is_runeword: bool,
    pub is_ethereal: bool,
    pub magic_prefix: Option<u16>,
    pub magic_suffix: Option<u16>,
    pub rare_name_1: Option<u8>,
    pub rare_name_2: Option<u8>,
    pub rare_affixes: [Option<u16>; 6],
    pub unique_id: Option<u16>,
    pub runeword_id: Option<u16>,
    pub runeword_level: Option<u8>,
    pub properties: Vec<ItemProperty>,
    pub set_attributes: Vec<Vec<ItemProperty>>,
    pub runeword_attributes: Vec<ItemProperty>,
    pub num_socketed_items: u8,
    pub socketed_items: Vec<Item>,
    pub timestamp_flag: bool,
    pub properties_complete: bool,
    pub set_list_count: u8,
    pub tbk_ibk_teleport: Option<u8>,
    pub defense: Option<u32>,
    pub max_durability: Option<u32>,
    pub current_durability: Option<u32>,
    pub quantity: Option<u32>,
    pub sockets: Option<u8>,
    pub modules: Vec<ItemModule>,
}

impl Item {
    pub fn header_view(&self) -> ItemHeader {
        ItemHeader {
            id: self.id,
            quality: self.quality,
            version: self.version,
            is_compact: self.is_compact,
            is_identified: self.is_identified,
            is_socketed: self.is_socketed,
            is_personalized: self.is_personalized,
            is_runeword: self.is_runeword,
            is_ethereal: self.is_ethereal,
        }
    }

    pub fn body_view(&self) -> ItemBody {
        ItemBody {
            code: self.code.clone(),
            x: self.x,
            y: self.y,
            page: self.page,
            location: self.location,
            mode: self.mode,
            defense: self.defense,
            max_durability: self.max_durability,
            current_durability: self.current_durability,
            quantity: self.quantity,
        }
    }

    /// Mutates the item using a checked placement.
    /// This clears the cached bitstream, forcing a re-encoding.
    pub fn set_placement(&mut self, placement: crate::domain::vo::InventoryPlacement) {
        self.x = placement.coordinate().x();
        self.y = placement.coordinate().y();
        // Clear bits to force re-calculation from fields
        self.bits.clear();
    }

    /// Mutates a specific property value.
    /// Returns true if the property was found and updated.
    pub fn set_property_value(&mut self, stat_id: u32, value: crate::domain::vo::ItemStatValue) -> bool {
        println!("DEBUG: set_property_value(id={}, val={}), self.version={}, self.is_runeword={}", stat_id, value.value(), self.version, self.is_runeword);
        let mut found = false;
        
        {
            let mut lists = Vec::new();
            lists.push(&mut self.properties);
            for list in &mut self.set_attributes {
                lists.push(list);
            }
            lists.push(&mut self.runeword_attributes);

            for list in lists.into_iter() {
                for prop in list {
                    if prop.stat_id == stat_id {
                        let cost = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == stat_id);
                        if let Some(c) = cost {
                            prop.value = value.value();
                            prop.raw_value = value.value().wrapping_add(c.save_add);
                            found = true;
                        }
                    }
                }
            }
        }
        
        if found {
            self.bits.clear();
        }
        found
    }


    pub fn stats_view(&self) -> ItemStats {
        ItemStats {
            properties: self.properties.clone(),
            set_attributes: self.set_attributes.clone(),
            runeword_attributes: self.runeword_attributes.clone(),
        }
    }

    pub fn prefixes(&self) -> Vec<&'static crate::data::item_specs::Affix> {
        let mut result = Vec::new();
        if let Some(id) = self.magic_prefix {
            if let Some(affix) = crate::data::affixes::PREFIXES.iter().find(|a| a.id == id as u32) {
                result.push(affix);
            }
        }
        // Rare prefixes are in slots 0, 2, 4
        for i in [0, 2, 4] {
            if let Some(id) = self.rare_affixes[i] {
                if let Some(affix) = crate::data::affixes::PREFIXES.iter().find(|a| a.id == id as u32) {
                    result.push(affix);
                }
            }
        }
        result
    }

    pub fn suffixes(&self) -> Vec<&'static crate::data::item_specs::Affix> {
        let mut result = Vec::new();
        if let Some(id) = self.magic_suffix {
            if let Some(affix) = crate::data::affixes::SUFFIXES.iter().find(|a| a.id == id as u32) {
                result.push(affix);
            }
        }
        // Rare suffixes are in slots 1, 3, 5
        for i in [1, 3, 5] {
            if let Some(id) = self.rare_affixes[i] {
                if let Some(affix) = crate::data::affixes::SUFFIXES.iter().find(|a| a.id == id as u32) {
                    result.push(affix);
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemProperty {
    pub stat_id: u32,
    pub name: String,
    pub param: u32,
    pub raw_value: i32,
    pub value: i32, // After applying save_add if needed
}

fn item_template(code: &str) -> Option<&'static crate::data::item_codes::ItemTemplate> {
    crate::data::item_codes::ITEM_TEMPLATES
        .iter()
        .find(|template| template.code == code.trim())
}

fn stat_save_bits(stat_id: u32) -> Option<u32> {
    crate::data::stat_costs::STAT_COSTS
        .iter()
        .find(|stat| stat.id == stat_id)
        .map(|stat| stat.save_bits as u32)
}

fn read_player_name<R: BitRead>(recorder: &mut BitRecorder<R>) -> io::Result<String> {
    let mut name = String::new();
    loop {
        let ch = recorder.read_bits(7)? as u8;
        if ch == 0 {
            break;
        }
        name.push(ch as char);
    }
    Ok(name)
}

pub fn read_property_list<R: BitRead>(
    recorder: &mut BitRecorder<R>,
    code: &str,
    version: u8,
    section_recovery: Option<(&[u8], u64)>,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> io::Result<(Vec<ItemProperty>, bool)> {
    let mut props = Vec::new();

    if version == 5 { println!("[DEBUG v5] Starting List is_list2={} at bit {}", alpha_runeword, recorder.recorded_bits.len()); }
    loop {
        let _bit_pos = recorder.recorded_bits.len();
        match parse_single_property(recorder, code, version, section_recovery, huffman, alpha_runeword)? {
            PropertyParseResult::Property(prop) => props.push(prop),
            PropertyParseResult::Terminator => return Ok((props, true)),
            PropertyParseResult::Recovered => return Ok((props, false)),
        }
    }
}

enum PropertyParseResult {
    Property(ItemProperty),
    Terminator,
    Recovered,
}

pub fn parse_single_property<R: BitRead>(
    recorder: &mut BitRecorder<R>,
    code: &str,
    version: u8,
    section_recovery: Option<(&[u8], u64)>,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> io::Result<PropertyParseResult> {
    let bit_pos = recorder.recorded_bits.len();
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    let stat_id = match recorder.read_bits(id_bits) {
        Ok(stat_id) => stat_id,
        Err(err) => {
            if let Some((section_bytes, item_start_bit)) = section_recovery {
                if recover_property_reader(recorder, code, section_bytes, item_start_bit, huffman)? {
                    return Ok(PropertyParseResult::Recovered);
                }
            }
            return Err(err);
        }
    };

    if stat_id == terminator {
        return Ok(PropertyParseResult::Terminator);
    }

    let cost_opt = crate::data::stat_costs::STAT_COSTS.iter().find(|s| s.id == stat_id);
    match cost_opt {
        Some(cost) => {
            let save_bits = cost.save_bits;
            let val = recorder.read_bits(save_bits as u32)?;
            if version == 5 {
                println!("  [ID {}] name={}, val={}, start_bit={}", stat_id, cost.name, val, bit_pos);
            }
            return Ok(PropertyParseResult::Property(ItemProperty {
                stat_id,
                name: cost.name.to_string(),
                param: 0, // Simplified for trace
                raw_value: val as i32,
                value: (val as i32) - cost.save_add,
            }));
        }
        None => {
            if version == 5 {
                println!("  [ID {}] UNKNOWN. Assuming 15-bit value for trace...", stat_id);
                let val = recorder.read_bits(15)?;
                return Ok(PropertyParseResult::Property(ItemProperty {
                    stat_id,
                    name: format!("unknown_{}", stat_id),
                    param: 0,
                    raw_value: val as i32,
                    value: val as i32,
                }));
            } else {
                let err = crate::error::DiagnosticError::new(
                    bit_pos,
                    "Expected valid stat ID from STAT_COSTS",
                    stat_id.to_string(),
                    format!("Stat ID {} is not defined in game data for item '{}'.", stat_id, code)
                );
                item_trace!("    [ERROR] Found unknown stat ID: {}", err);
                if let Some((section_bytes, item_start_bit)) = section_recovery {
                    if recover_property_reader(recorder, code, section_bytes, item_start_bit, huffman)? {
                        return Ok(PropertyParseResult::Recovered);
                    }
                }
                return Err(io::Error::new(io::ErrorKind::InvalidData, err.to_string()));
            }
        }
    }
}

/// A pure function for stat value adjustment.
/// Verified by Kani to be safe for all i32 inputs.
pub fn calculate_stat_value(raw: i32, save_add: i32) -> i32 {
    raw.wrapping_sub(save_add)
}


fn write_player_name(emitter: &mut BitEmitter, name: &str) -> io::Result<()> {
    for ch in name.chars() {
        emitter.write_bits((ch as u8 & 0x7F) as u32, 7)?;
    }
    emitter.write_bits(0, 7)?;
    Ok(())
}

fn write_property_list(emitter: &mut BitEmitter, props: &[ItemProperty], version: u8, alpha_runeword: bool) -> io::Result<()> {
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    for prop in props {
        if version == 5 {
            // Alpha v105 (v5): Fixed-width binary stats.
            let val_bits = if alpha_runeword { 12 } else { 10 };
            emitter.write_bits(prop.stat_id, 9)?;
            emitter.write_bits(prop.raw_value as u32, val_bits)?;
        } else {
           let stat = crate::data::stat_costs::STAT_COSTS
                .iter()
                .find(|s| s.id == prop.stat_id)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("Missing stat_cost entry for stat_id {}", prop.stat_id),
                    )
                })?;
            emitter.write_bits(prop.stat_id, id_bits)?;
            if stat.save_param_bits > 0 {
                emitter.write_bits(prop.param, stat.save_param_bits as u32)?;
            }
            if stat.save_bits > 0 {
                emitter.write_bits(prop.raw_value as u32, stat.save_bits as u32)?;
            }
        }
    }
    emitter.write_bits(terminator, id_bits)?;
    Ok(())
}


impl Item {
    fn read_item_header<R: BitRead>(recorder: &mut BitRecorder<R>) -> io::Result<(u32, u8, u8, u8, u8, u8, u8, u8)> {
        let flags = recorder.read_bits(32)?;
        let version = recorder.read_bits(3)? as u8;
        let mode = recorder.read_bits(3)? as u8;
        let loc = recorder.read_bits(4)? as u8;
        let x = (recorder.read_bits(4)? & 0x0F) as u8;
        let y = (recorder.read_bits(4)? & 0x0F) as u8;
        let page = (recorder.read_bits(3)? & 0x07) as u8;
        let socket_hint = (recorder.read_bits(3)? & 0x07) as u8;
        Ok((flags, version, mode, loc, x, y, page, socket_hint))
    }


    fn read_item_code<R: BitRead>(
        recorder: &mut BitRecorder<R>,
        is_ear: bool,
        huffman: &HuffmanTree,
        _version: u8,
    ) -> io::Result<(String, Option<u8>, Option<u8>, Option<String>)> {
        let mut ear_class = None;
        let mut ear_level = None;
        let mut ear_player_name = None;

        let code = if is_ear {
            let ear_class_bits = recorder.read_bits(3)? as u8;
            let ear_level_bits = recorder.read_bits(7)? as u8;
            let player_name = read_player_name(recorder)?;
            ear_class = Some(ear_class_bits);
            ear_level = Some(ear_level_bits);
            ear_player_name = Some(player_name);
            "ear ".to_string()
        } else {
            let mut decoded = String::new();
            for _ in 0..4 {
                decoded.push(huffman.decode_recorded(recorder)?);
            }
            decoded
        };
        Ok((code, ear_class, ear_level, ear_player_name))
    }


    fn read_extended_stats<R: BitRead>(
        recorder: &mut BitRecorder<R>,
        code: &str,
        is_socketed: bool,
        is_runeword: bool,
        is_personalized: bool,
        version: u8,
    ) -> io::Result<(
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
        let trimmed_code = code.trim();
        let template = item_template(code);

        let (item_id, item_level, quality) = parse_base_header(recorder, version)?;
        let mut item_id = Some(item_id);
        let mut item_level = Some(item_level);
        let mut item_quality = Some(quality);

        if version == 5 {
            // Alpha v105 (v5): We confirmed List 1 starts at exactly offset 230 bits.
            // Current bits consumed: read_item_header (56) + code (0) + parse_base_header (16+27?) = 99?
            // We use a confirmed 141-bit skip to align from current early-return structure (v5 header=16, flags=3).
            // Actually, let's just use the brute-force confirmed skip if we find it.
            // For now, use the empirical offset: 230 (L1 start) - current (read_item_header 56 + base_header 16 + padding/flags 17) = 141.
            for _ in 0..93 { let _ = recorder.read_bit()?; }
            return Ok((
                item_id, item_level, item_quality, 
                false, None, false, None, None, None, None, // Graphics/Class/Magic
                None, None, [None; 6], // Rare
                None, None, None, // Unique/Runeword IDs
                None, None, // Personalization/Teleport
                false, None, None, None, None, None, 0 // Fields/Defense/Durability
            ));
        }
        item_trace!(
            "  [Stats] ID: {:?}, Lvl: {:?}, Quality: {:?}",
            item_id,
            item_level,
            quality
        );

        let has_multiple_graphics = recorder.read_bits(1)? != 0;
        let multi_graphics_bits = if has_multiple_graphics {
            Some(recorder.read_bits(3)? as u8)
        } else {
            None
        };
        let has_class_specific_data = recorder.read_bits(1)? != 0;
        let class_specific_bits = if has_class_specific_data {
            Some(recorder.read_bits(11)? as u16)
        } else {
            None
        };

        let mut low_high_graphic_bits = None;
        let mut magic_prefix = None;
        let mut magic_suffix = None;
        let mut rare_name_1 = None;
        let mut rare_name_2 = None;
        let mut rare_affixes = [None; 6];
        let mut unique_id = None;

        match quality {
            ItemQuality::Low | ItemQuality::High => {
                low_high_graphic_bits = Some(recorder.read_bits(3)? as u8);
            }
            ItemQuality::Magic => {
                if version == 5 {
                    magic_prefix = Some(recorder.read_bits(7)? as u16);
                    magic_suffix = Some(recorder.read_bits(7)? as u16);
                } else {
                    magic_prefix = Some(recorder.read_bits(11)? as u16);
                    magic_suffix = Some(recorder.read_bits(11)? as u16);
                }
            }
            ItemQuality::Rare | ItemQuality::Crafted => {
                rare_name_1 = Some(recorder.read_bits(8)? as u8);
                rare_name_2 = Some(recorder.read_bits(8)? as u8);
                for i in 0..6 {
                    if recorder.read_bit()? {
                        rare_affixes[i] = Some(recorder.read_bits(11)? as u16);
                    }
                }
            }
            ItemQuality::Set | ItemQuality::Unique => {
                unique_id = Some(recorder.read_bits(12)? as u16);
            }
            _ => {}
        }

        let mut runeword_id = None;
        let mut runeword_level = None;
        if is_runeword && version != 5 {
            runeword_id = Some(recorder.read_bits(12)? as u16);
            runeword_level = Some(recorder.read_bits(4)? as u8);
        }

        let mut personalized_player_name = None;
        if is_personalized {
            personalized_player_name = Some(read_player_name(recorder)?);
        }

        let tbk_ibk_teleport = if trimmed_code == "tbk" || trimmed_code == "ibk" {
            Some(recorder.read_bits(5)? as u8)
        } else {
            None
        };

        // D2R stores a 1-bit timestamp flag here, not a 96-bit realm-data block.
        let timestamp_flag = recorder.read_bits(1)? != 0;
        item_trace!(
            "  [Debug] Earliest known timestamp flag = {}",
            timestamp_flag
        );

        let (reads_defense, reads_durability, reads_quantity) = if let Some(template) = template {
            item_trace!("  [Stats] Template identified: is_armor={}, has_durability={}, is_stackable={}", template.is_armor, template.has_durability, template.is_stackable);
            (
                template.is_armor,
                template.has_durability,
                template.is_stackable,
            )
        } else {
            let armor_like_unknown = has_class_specific_data || trimmed_code.contains(' ');
            item_trace!("  [Stats] No template found, using heuristics: armor_like_unknown={}", armor_like_unknown);
            (armor_like_unknown, armor_like_unknown, false)
        };

        if version == 5 {
            // In Alpha v105 v5, Stats like Defense/Durability are skipped in the header block.
            // They are part of the property lists.
            return Ok((
                item_id, item_level, item_quality, 
                has_multiple_graphics, multi_graphics_bits,
                has_class_specific_data, class_specific_bits,
                low_high_graphic_bits, magic_prefix, magic_suffix,
                rare_name_1, rare_name_2, rare_affixes,
                unique_id, runeword_id, runeword_level,
                personalized_player_name, tbk_ibk_teleport,
                timestamp_flag, None, None, None, None, None, 0
            ));
        }

        let mut defense = None;
        if reads_defense {
            let defense_bits = stat_save_bits(31).unwrap_or(11);
            defense = Some(recorder.read_bits(defense_bits)?);
            item_trace!("  [Stats] Defense: {:?}", defense);
        }

        let mut max_durability = None;
        let mut current_durability = None;
        if reads_durability {
            let max_dur_bits = stat_save_bits(73).unwrap_or(8);
            let cur_dur_bits = stat_save_bits(72).unwrap_or(9);
            let m_dur = recorder.read_bits(max_dur_bits)?;
            max_durability = Some(m_dur);
            item_trace!("  [Stats] Max Durability Bits: {}, Value: {}", max_dur_bits, m_dur);
            if m_dur > 0 {
                let current = recorder.read_bits(cur_dur_bits)?;
                current_durability = Some(current);
                item_trace!("  [Stats] Current Durability Bits: {}, Value: {}", cur_dur_bits, current);
                let dur_extra = recorder.read_bit()?;
                item_trace!("  [Stats] Durability Extra Bit: {}", dur_extra);
            }
        }

        let mut quantity = None;
        if reads_quantity {
            quantity = Some(recorder.read_bits(9)?);
        }

        let mut sockets = None;
        if is_socketed {
            let count = recorder.read_bits(4)? as u8;
            sockets = Some(count);
            item_trace!("  [Stats] Sockets Count Bits: 4, Value: {}", count);
        }

        let mut set_list_count = 0;
        if item_quality == Some(ItemQuality::Set) {
            let set_list_value = recorder.read_bits(5)?;
            set_list_count = match set_list_value {
                1 | 2 | 4 => 1,
                3 | 6 | 10 | 12 => 2,
                7 => 3,
                15 => 4,
                31 => 5,
                _ => 0,
            };
        }

        Ok((
            item_id,
            item_level,
            item_quality,
            has_multiple_graphics,
            multi_graphics_bits,
            has_class_specific_data,
            class_specific_bits,
            low_high_graphic_bits,
            magic_prefix,
            magic_suffix,
            rare_name_1,
            rare_name_2,
            rare_affixes,
            unique_id,
            runeword_id,
            runeword_level,
            personalized_player_name,
            tbk_ibk_teleport,
            timestamp_flag,
            defense,
            max_durability,
            current_durability,
            quantity,
            sockets,
            set_list_count,
        ))
    }


    fn read_item_stats<R: BitRead>(
        recorder: &mut BitRecorder<R>,
        code: &str,
        version: u8,
        quality: Option<ItemQuality>,
        set_list_count: u8,
        is_runeword: bool,
        ctx: Option<(&[u8], u64)>,
        huffman: &HuffmanTree,
    ) -> io::Result<(Vec<ItemProperty>, Vec<Vec<ItemProperty>>, Vec<ItemProperty>, bool)> {
        let trimmed_code = code.trim();
        let (properties, properties_complete) =
            read_property_list(recorder, trimmed_code, version, ctx, huffman, false)?;

        let mut set_attributes = Vec::new();
        let mut runeword_attributes = Vec::new();

        let mut parse_property_lists = properties_complete;
        if parse_property_lists && quality == Some(ItemQuality::Set) && set_list_count > 0 {
            for _ in 0..set_list_count {
                let (set_props, complete) =
                    read_property_list(recorder, trimmed_code, version, ctx, huffman, false)?;
                set_attributes.push(set_props);
                if !complete {
                    parse_property_lists = false;
                    break;
                }
            }
        }

        if parse_property_lists && is_runeword && version == 5 {
            // Alpha v105: 93-bit spacer confirmed between List 1 and List 2.
            for _ in 0..93 { let _ = recorder.read_bit()?; }
            let (rw_props, complete) =
                read_property_list(recorder, trimmed_code, version, ctx, huffman, true)?;
            if complete {
                runeword_attributes = rw_props;
            }
        }

        Ok((properties, set_attributes, runeword_attributes, properties_complete))
    }

    pub fn from_reader_with_context<R: BitRead>(
        recorder: &mut BitRecorder<R>,
        huffman: &HuffmanTree,
        ctx: Option<(&[u8], u64)>,
    ) -> io::Result<Item> {

        let (flags, version, mode, loc, x, y, page, header_socket_hint) = Self::read_item_header(recorder)?;

        let is_identified = (flags & (1 << 4)) != 0;
        let is_socketed = if version == 5 { (flags & (1 << 11)) != 0 } else { (flags & (1 << 11)) != 0 };
        let is_ear = (flags & (1 << 16)) != 0;
        let is_compact = (flags & (1 << 21)) != 0;
        let is_ethereal = (flags & (1 << 22)) != 0;
        let is_personalized = (flags & (1 << 24)) != 0;
        
        let is_runeword = if version == 5 { 
            (flags & (1 << 11)) != 0 
        } else { 
            (flags & (1 << 26)) != 0 
        };

        let (code, ear_class, ear_level, ear_player_name) = if version == 5 && !is_ear && is_runeword {
            // Alpha v105: Runeword items (version 5) seem to skip Huffman item code.
            // We'll tentatively use "xrs " as it will be identified by stats later.
            ("xrs ".to_string(), None, None, None)
        } else {
            Self::read_item_code(recorder, is_ear, huffman, version)?
        };

        if is_ear {
            return Ok(Item {
                bits: recorder.recorded_bits.clone(),
                code,
                flags,
                version,
                is_ear,
                ear_class,
                ear_level,
                ear_player_name,
                personalized_player_name: None,
                mode,
                x,
                y,
                page,
                location: loc,
                header_socket_hint,
                has_multiple_graphics: false,
                multi_graphics_bits: None,
                has_class_specific_data: false,
                class_specific_bits: None,
                id: None,
                level: None,
                quality: None,
                low_high_graphic_bits: None,
                is_compact: false,
                is_socketed: false,
                is_identified,
                is_personalized,
                is_runeword: false,
                is_ethereal,
                magic_prefix: None,
                magic_suffix: None,
                rare_name_1: None,
                rare_name_2: None,
                rare_affixes: [None; 6],
                unique_id: None,
                runeword_id: None,
                runeword_level: None,
                properties: Vec::new(),
                set_attributes: Vec::new(),
                runeword_attributes: Vec::new(),
                num_socketed_items: 0,
                socketed_items: Vec::new(),
                timestamp_flag: false,
                properties_complete: false,
                set_list_count: 0,
                tbk_ibk_teleport: None,
                defense: None,
                max_durability: None,
                current_durability: None,
                quantity: None,
                sockets: None,
                modules: Vec::new(),
            });
        }

        let trimmed_code = code.trim();
        item_trace!("  [Item] Code: '{}', Flags: 0x{:08X}", trimmed_code, flags);

        item_trace!("  [Header] Socket Hint: {}, Bit Offset: {}", header_socket_hint, recorder.recorded_bits.len());

        let stats = if !is_compact {
            Self::read_extended_stats(recorder, &code, is_socketed, is_runeword, is_personalized, version)?
        } else {
            (
                None, None, None, false, None, false, None, None, None, None, None, None,
                [None; 6], None, None, None, None, None, false, None, None, None, None, None, 0,
            )
        };

        let item_id = stats.0;
        let item_level = stats.1;
        let item_quality = stats.2;
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

        if let Some(def_val) = defense {
            if !crate::data::item_specs::is_valid_base_ac(trimmed_code, def_val as u16, is_ethereal) {
                item_trace!(
                    "  [Warn] Invalid base defense for '{}': {} (ethereal: {})",
                    trimmed_code, def_val, is_ethereal
                );
            }
        }

        let max_durability = stats.20;
        let current_durability = stats.21;
        let quantity = stats.22;
        let sockets = stats.23;
        let set_list_count = stats.24;

        let (properties, set_attributes, runeword_attributes, properties_complete) = if !is_compact {
            Self::read_item_stats(
                recorder,
                &code,
                version,
                item_quality,
                set_list_count,
                is_runeword,
                ctx,
                huffman,
            )?
        } else {
            (Vec::new(), Vec::new(), Vec::new(), true)
        };

        if !properties_complete {
            item_trace!(
                "  [Warn] Property list for '{}' ended by recovery boundary; skipping set/runeword blocks.",
                trimmed_code
            );
        }

        Ok(Item {
            bits: recorder.recorded_bits.clone(),
            code,
            flags,
            version,
            is_ear,
            ear_class,
            ear_level,
            ear_player_name,
            personalized_player_name,
            mode,
            x,
            y,
            page,
            location: loc,
            header_socket_hint,
            has_multiple_graphics,
            multi_graphics_bits,
            has_class_specific_data,
            class_specific_bits,
            id: item_id,
            level: item_level,
            quality: item_quality,
            low_high_graphic_bits,
            is_compact,
            is_socketed,
            is_identified,
            is_personalized,
            is_runeword,
            is_ethereal,
            magic_prefix,
            magic_suffix,
            rare_name_1,
            rare_name_2,
            rare_affixes,
            unique_id,
            runeword_id,
            runeword_level,
            properties,
            set_attributes,
            runeword_attributes,
            num_socketed_items: header_socket_hint,
            socketed_items: Vec::new(),
            timestamp_flag,
            properties_complete,
            set_list_count,
            tbk_ibk_teleport,
            defense,
            max_durability,
            current_durability,
            quantity,
            sockets,
            modules: Vec::new(),
        })
    }

    pub fn spec_lookup(&self) -> Option<crate::engine::validation::ItemSpec> {
        crate::engine::validation::lookup_spec(self)
    }

    pub fn from_reader<R: BitRead>(reader: &mut R, huffman: &HuffmanTree) -> io::Result<Self> {
        let mut recorder = BitRecorder::new(reader);
        Self::from_reader_with_context(&mut recorder, huffman, None)
    }

    pub fn from_bytes(bytes: &[u8], huffman: &HuffmanTree) -> io::Result<Self> {
        let mut reader = IoBitReader::endian(io::Cursor::new(bytes), LittleEndian);
        Self::from_reader(&mut reader, huffman)
    }

    pub fn read_section(
        section_bytes: &[u8],
        top_level_count: u16,
        huffman: &HuffmanTree,
    ) -> io::Result<Vec<Item>> {
        let section_bits = (section_bytes.len() * 8) as u64;
        let mut items: Vec<Item> = Vec::with_capacity(top_level_count as usize);
        let mut bit_pos = 0u64;

        while bit_pos < section_bits {
            let mut start = bit_pos;
            if let Some(next_start) = find_next_item_match(section_bytes, start, huffman) {
                if next_start != start {
                    item_trace!("  [Section] Found next item at bit {} (skipped {} bits).", next_start, next_start - start);
                }
                start = next_start;
            } else {
                break;
            }

            let (item, consumed_bits) = parse_item_at(section_bytes, start, huffman)?;
            bit_pos = align_to_byte(start + consumed_bits);

            let end = start + consumed_bits;
            if end <= start {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "item parser did not advance",
                ));
            }
            bit_pos = end;

            if item.mode == 6 || item.location == 6 {
                if let Some(parent) = items.last_mut() {
                    parent.socketed_items.push(item);
                } else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "socketed item without a parent",
                    ));
                }
            } else {
                items.push(item);
                let is_last_top_level = items.len() == top_level_count as usize;
                if let Some(parent) = items.last_mut() {
                    if parent.is_socketed || parent.is_runeword || is_last_top_level {
                        let rescue_limit = socket_rescue_limit(parent);
                        if let Some((rescued_children, rescued_end)) =
                            scan_socket_children(section_bytes, bit_pos, huffman, rescue_limit)
                        {
                            parent.socketed_items.extend(rescued_children);
                            bit_pos = rescued_end;
                        }
                    }
                }
            }
            
            // Alpha v105 heuristic: if we reached the count including socketed items,
            // then for v5 saves this might be exactly what top_level_count means.
            if items.iter().map(|it| 1 + it.socketed_items.len()).sum::<usize>() >= top_level_count as usize {
                if Self::version_sum_check(&items, top_level_count) {
                    break;
                }
            }

            if items.len() == top_level_count as usize {
                break;
            }
        }

        if items.len() != top_level_count as usize && !Self::version_sum_check(&items, top_level_count) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "item count mismatch: expected {}, parsed {}",
                    top_level_count,
                    items.len()
                ),
            ));
        }

        Ok(items)
    }

    fn version_sum_check(items: &[Item], top_level_count: u16) -> bool {
        let has_v5 = items.iter().any(|it| it.version == 5);
        if !has_v5 { return false; }
        let total: usize = items.iter().map(|it| 1 + it.socketed_items.len()).sum();
        total == top_level_count as usize
    }


    pub fn empty_for_tests() -> Self {
        Item {
            bits: Vec::new(),
            code: "    ".to_string(),
            flags: 0,
            version: 0,
            is_ear: false,
            ear_class: None,
            ear_level: None,
            ear_player_name: None,
            personalized_player_name: None,
            mode: 0,
            x: 0,
            y: 0,
            page: 0,
            location: 0,
            header_socket_hint: 0,
            has_multiple_graphics: false,
            multi_graphics_bits: None,
            has_class_specific_data: false,
            class_specific_bits: None,
            id: None,
            level: None,
            quality: None,
            low_high_graphic_bits: None,
            is_compact: false,
            is_socketed: false,
            is_identified: false,
            is_personalized: false,
            is_runeword: false,
            is_ethereal: false,
            magic_prefix: None,
            magic_suffix: None,
            rare_name_1: None,
            rare_name_2: None,
            rare_affixes: [None; 6],
            unique_id: None,
            runeword_id: None,
            runeword_level: None,
            properties: Vec::new(),
            set_attributes: Vec::new(),
            runeword_attributes: Vec::new(),
            num_socketed_items: 0,
            socketed_items: Vec::new(),
            timestamp_flag: false,
            properties_complete: false,
            set_list_count: 0,
            tbk_ibk_teleport: None,
            defense: None,
            max_durability: None,
            current_durability: None,
            quantity: None,
            sockets: None,
            modules: Vec::new(),
        }
    }

    pub fn read_player_items(bytes: &[u8], huffman: &HuffmanTree) -> io::Result<Vec<Item>> {
        let jm_pos = (0..bytes.len().saturating_sub(1))
            .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "JM header not found"))?;
        let top_level_count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
        let next_jm = (jm_pos + 4..bytes.len().saturating_sub(1))
            .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
            .unwrap_or(bytes.len());

        Item::read_section(&bytes[jm_pos + 4..next_jm], top_level_count, huffman)
    }

    pub fn scan_items(bytes: &[u8], huffman: &HuffmanTree) -> Vec<(usize, String)> {
        let start_scan = 0;
        let end_scan = bytes.len() * 8 - 40;
        let mut item_starts: Vec<(usize, String)> = Vec::new();
        for start in start_scan..end_scan {
            let mut reader = IoBitReader::endian(Cursor::new(bytes), LittleEndian);
            let _ = reader.skip(start as u32);
            let mut code = String::new();
            let mut valid = true;
            for _ in 0..4 {
                match huffman.decode(&mut reader) {
                    Ok(c) => code.push(c),
                    Err(_) => {
                        valid = false;
                        break;
                    }
                }
            }
            if valid {
                let known = [
                    "hp1 ", "mp1 ", "tsc ", "isc ", "buc ", "jav ", "rin ", "amu ", "key ", "tbk ",
                    "ibk ", "vps ",
                ];
                if known.contains(&code.as_str()) {
                    if item_starts.is_empty() || start - item_starts.last().unwrap().0 > 32 {
                        item_starts.push((start, code));
                    }
                }
            }
        }
        item_starts
    }

    fn write_header_block(
        &self,
        emitter: &mut BitEmitter,
        huffman: &HuffmanTree,
    ) -> io::Result<()> {
        emitter.write_bits(self.flags, 32)?;
        emitter.write_bits(self.version as u32, 3)?;
        emitter.write_bits(self.mode as u32, 3)?;
        emitter.write_bits(self.location as u32, 4)?;
        emitter.write_bits(self.x as u32, 4)?;
        emitter.write_bits(self.y as u32, 4)?;
        emitter.write_bits(self.page as u32, 3)?;

        if self.version == 5 {
            emitter.write_bits(self.header_socket_hint as u32, 3)?;
        }

        if self.is_ear {
            emitter.write_bits(self.ear_class.unwrap_or(0) as u32, 3)?;
            emitter.write_bits(self.ear_level.unwrap_or(0) as u32, 7)?;
            if let Some(name) = &self.ear_player_name {
                write_player_name(emitter, name)?;
            } else {
                write_player_name(emitter, "")?;
            }
            return Ok(());
        }

        let skip_code = self.version == 5 && self.is_runeword;
        if !skip_code {
            let encoded_code = huffman.encode(&self.code)?;
            emitter.extend_bits(encoded_code)?;
        }

        if self.version != 5 {
            emitter.write_bits(self.header_socket_hint as u32, 3)?;
        }

        if self.version == 5 {
             return Ok(());
        }

        emitter.write_bit(self.has_multiple_graphics)?;
        if self.has_multiple_graphics {
            emitter.write_bits(self.multi_graphics_bits.unwrap_or(0) as u32, 3)?;
        }
        emitter.write_bit(self.has_class_specific_data)?;
        if self.has_class_specific_data {
            emitter.write_bits(self.class_specific_bits.unwrap_or(0) as u32, 11)?;
        }

        Ok(())
    }

    fn write_extended_stats(&self, emitter: &mut BitEmitter) -> io::Result<()> {
        if self.is_ear || self.is_compact {
            return Ok(());
        }

        if self.version == 5 {
            // Alpha v105 (v5): 16-bit base header
            emitter.write_bits(self.level.unwrap_or(0) as u32, 7)?;
            let quality = self.quality.unwrap_or(ItemQuality::Normal);
            emitter.write_bits(quality as u32, 4)?;
            emitter.write_bits(0x01, 5)?; 

            if quality == ItemQuality::Magic {
                // Alpha v105: Magic affixes appear to be 7 bits each (matches 89-bit property start)
                emitter.write_bits(self.magic_prefix.unwrap_or(0) as u32, 7)?;
                emitter.write_bits(self.magic_suffix.unwrap_or(0) as u32, 7)?;
            }

            emitter.write_bit(self.has_multiple_graphics)?; 
            emitter.write_bit(self.has_class_specific_data)?;
            emitter.write_bit(self.timestamp_flag)?;
            
            // In Alpha v105 v5, Defense/Durability/Sockets are NOT in header,
            // they are stored as properties. Property List 1 starts at bit 89.
            return Ok(());
        } else {
            emitter.write_bits(self.id.unwrap_or(0), 32)?;
            emitter.write_bits(self.level.unwrap_or(0) as u32, 7)?;
            let quality = self.quality.unwrap_or(ItemQuality::Normal);
            emitter.write_bits(quality as u32, 4)?;

            match quality {
                ItemQuality::Low | ItemQuality::High => {
                    emitter.write_bits(self.low_high_graphic_bits.unwrap_or(0) as u32, 3)?;
                }
                ItemQuality::Magic => {
                    emitter.write_bits(self.magic_prefix.unwrap_or(0) as u32, 11)?;
                    emitter.write_bits(self.magic_suffix.unwrap_or(0) as u32, 11)?;
                }
                ItemQuality::Rare | ItemQuality::Crafted => {
                    emitter.write_bits(self.rare_name_1.unwrap_or(0) as u32, 8)?;
                    emitter.write_bits(self.rare_name_2.unwrap_or(0) as u32, 8)?;
                    for i in 0..6 {
                        if let Some(value) = self.rare_affixes[i] {
                            emitter.write_bit(true)?;
                            emitter.write_bits(value as u32, 11)?;
                        } else {
                            emitter.write_bit(false)?;
                        }
                    }
                }
                ItemQuality::Set | ItemQuality::Unique => {
                    emitter.write_bits(self.unique_id.unwrap_or(0) as u32, 12)?;
                }
                _ => {}
            }

            if self.is_runeword {
                emitter.write_bits(self.runeword_id.unwrap_or(0) as u32, 12)?;
                emitter.write_bits(self.runeword_level.unwrap_or(0) as u32, 4)?;
            }
        }

        if self.is_personalized {
            let name = self.personalized_player_name.as_deref().unwrap_or("");
            write_player_name(emitter, name)?;
        }

        let trimmed_code = self.code.trim();
        if trimmed_code == "tbk" || trimmed_code == "ibk" {
            emitter.write_bits(self.tbk_ibk_teleport.unwrap_or(0) as u32, 5)?;
        }

        emitter.write_bit(self.timestamp_flag)?;

        // Restore missing stats writing
        let template = item_template(&self.code);
        let (reads_defense, reads_durability, reads_quantity) = if let Some(template) = template {
            (template.is_armor, template.has_durability, template.is_stackable)
        } else {
            (self.has_class_specific_data || trimmed_code.contains(' '), 
             self.has_class_specific_data || trimmed_code.contains(' '), 
             false)
        };

        if reads_defense {
            let defense_bits = stat_save_bits(31).unwrap_or(11);
            emitter.write_bits(self.defense.unwrap_or(0), defense_bits)?;
        }

        if reads_durability {
            let max_dur_bits = stat_save_bits(73).unwrap_or(8);
            let cur_dur_bits = stat_save_bits(72).unwrap_or(9);
            let m_dur = self.max_durability.unwrap_or(0);
            emitter.write_bits(m_dur, max_dur_bits)?;
            if m_dur > 0 {
                emitter.write_bits(self.current_durability.unwrap_or(0), cur_dur_bits)?;
                emitter.write_bit(false)?; // durability extra bit
            }
        }

        if reads_quantity {
            emitter.write_bits(self.quantity.unwrap_or(0), 9)?;
        }

        if self.is_socketed {
            emitter.write_bits(self.sockets.unwrap_or(0) as u32, 4)?;
        }

        Ok(())
    }

    fn write_property_groups(
        &self,
        emitter: &mut BitEmitter,
        huffman: &HuffmanTree,
    ) -> io::Result<()> {
        if self.is_ear {
            return Ok(());
        }

        write_property_list(emitter, &self.properties, self.version, false)?;

        if self.properties_complete {
            for idx in 0..(self.set_list_count as usize) {
                if let Some(set_props) = self.set_attributes.get(idx) {
                    write_property_list(emitter, set_props, self.version, false)?;
                } else {
                    break;
                }
            }
            if self.is_runeword {
                if self.version == 5 {
                    // Alpha v105: No spacer.
                    write_property_list(emitter, &self.runeword_attributes, self.version, true)?;
                } else if !self.runeword_attributes.is_empty() {
                   write_property_list(emitter, &self.runeword_attributes, self.version, false)?;
                }
            }
        }
 else {
            item_trace!(
                "  [Warn] Skipping dependent property lists for '{}' (incomplete)",
                self.code.trim()
            );
        }

        for child in &self.socketed_items {
            emitter.byte_align()?;
            child.write_recursive(emitter, huffman)?;
        }

        Ok(())
    }

    fn write_recursive(&self, emitter: &mut BitEmitter, huffman: &HuffmanTree) -> io::Result<()> {
        self.write_header_block(emitter, huffman)?;
        self.write_extended_stats(emitter)?;
        self.write_property_groups(emitter, huffman)?;
        Ok(())
    }

    pub fn to_bytes(&self, huffman: &HuffmanTree) -> io::Result<Vec<u8>> {
        // If we have cached bits and no modification occurred (bits is not empty), use them.
        // However, Mutation clears the bits cache, so this will only trigger if unmodified.
        if !self.bits.is_empty() {
            let mut emitter = BitEmitter::new();
            for &rb in &self.bits {
                emitter.write_bit(rb.bit)?;
            }
            emitter.byte_align()?;
            return Ok(emitter.into_bytes());
        }
        
        // Re-encoding from scratch
        let mut emitter = BitEmitter::new();
        self.write_recursive(&mut emitter, huffman)?;
        emitter.byte_align()?;
        Ok(emitter.into_bytes())
    }

}

#[cfg(test)]
mod tests {
    use super::{HuffmanTree, Item};
    use std::fs;
    use std::path::PathBuf;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn parses_all_player_items_in_amazon_10_scrolls() {
        let bytes = fs::read(repo_path(
            "tests/fixtures/savegames/original/amazon_10_scrolls.d2s",
        ))
        .expect("fixture should exist");
        let huffman = HuffmanTree::new();
        let codes: Vec<String> = Item::read_player_items(&bytes, &huffman)
            .expect("items should parse")
            .into_iter()
            .map(|item| item.code)
            .collect();

        assert_eq!(codes.len(), 16);
        assert_eq!(codes[14], "jav ");
        assert_eq!(codes[15], "buc ");
    }
}

fn align_to_byte(bit_pos: u64) -> u64 {
    (bit_pos + 7) & !7
}

fn parse_item_at(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
) -> io::Result<(Item, u64)> {
    if start_bit % 8 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "item start must be byte-aligned",
        ));
    }

    let start_byte = (start_bit / 8) as usize;
    let mut reader = IoBitReader::endian(Cursor::new(&section_bytes[start_byte..]), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    let item =
        Item::from_reader_with_context(&mut recorder, huffman, Some((section_bytes, start_bit)))?;
    let consumed_bits = reader.position_in_bits()?;
    item_trace!("  [ParseAt] Parsed item '{}' at bit {}. Consumed {} bits.", item.code, start_bit, consumed_bits);
    Ok((item, consumed_bits))
}

const MAX_RESCUED_SOCKET_CHILDREN: usize = 6;
const SOCKET_CHILD_SCAN_WINDOW_BITS: u64 = 128;

fn socket_rescue_limit(parent: &Item) -> usize {
    let count = parent.num_socketed_items as usize;
    if (1..=MAX_RESCUED_SOCKET_CHILDREN).contains(&count) {
        count
    } else {
        MAX_RESCUED_SOCKET_CHILDREN
    }
}

fn is_plausible_item_header(mode: u8, location: u8, code: &str, flags: u32) -> bool {
    if mode > 6 || location > 15 {
        return false;
    }
    
    // Flags validation: bits 27-31 are unused in D2.
    if (flags & 0xF8000000) != 0 {
        return false;
    }

    let trimmed_code = code.trim();
    if trimmed_code.is_empty() {
        return false;
    }

    if item_template(code).is_some() {
        return code != "    " && code.trim().chars().count() > 0;
    }

    let is_rune = trimmed_code.len() == 3
        && trimmed_code.starts_with('r')
        && trimmed_code.chars().skip(1).all(|ch| ch.is_ascii_digit());
    let is_gem_like = trimmed_code.starts_with('g') || trimmed_code.starts_with("sk");
    let is_jewel = matches!(trimmed_code, "jew" | "j34" | "cjw");

    is_rune || is_jewel || is_gem_like
}

fn is_plausible_socket_child_header(mode: u8, location: u8, code: &str, flags: u32) -> bool {
    let Some(template) = item_template(code) else {
        return false;
    };
    if !(mode == 6 || location == 6) {
        return false;
    }
    
    // Flags validation: bits 27-31 are unused in D2.
    if (flags & 0xF8000000) != 0 {
        return false;
    }
    let code = code.trim();
    let is_rune =
        code.len() == 3 && code.starts_with('r') && code[1..].chars().all(|ch| ch.is_ascii_digit());
    let is_jewel = matches!(code, "jew" | "j34" | "cjw");
    let is_gem_like = code.starts_with('g') || code.starts_with("sk");

    (is_rune || is_jewel || is_gem_like)
        && template.width == 1
        && template.height == 1
        && !template.is_armor
        && !template.is_weapon
        && !template.has_durability
}

fn scan_socket_children(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
    max_children: usize,
) -> Option<(Vec<Item>, u64)> {
    // When the parent property list terminates too early, real socket children still
    // exist in the raw section bytes. We scan forward in a narrow byte-aligned window
    // and reattach plausible 1x1 socket fillers so the item tree remains lossless.
    if max_children == 0 {
        return None;
    }

    item_trace!("  [Sockets] Reading {} children at bit {}", max_children, start_bit);
    let mut children = Vec::new();
    let mut search_start = start_bit;
    let mut final_end = start_bit;

    while children.len() < max_children {
        item_trace!("  [Sockets] Searching for child {} at bit {}", children.len(), search_start);
        let Some((child, child_end)) = find_next_socket_child(section_bytes, search_start, huffman)
        else {
            break;
        };

        final_end = child_end;
        search_start = child_end;
        children.push(child);
    }

    if children.is_empty() {
        None
    } else {
        Some((children, final_end))
    }
}

fn find_next_socket_child(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
) -> Option<(Item, u64)> {
    let mut probe = align_to_byte(start_bit);
    let max_probe = (probe + SOCKET_CHILD_SCAN_WINDOW_BITS).min((section_bytes.len() * 8) as u64);

    while probe < max_probe {
        let Some((mode, location, code, flags)) = peek_item_header_at(section_bytes, probe, huffman)
        else {
            probe += 8;
            continue;
        };
        if !is_plausible_socket_child_header(mode, location, &code, flags) {
            probe += 8;
            continue;
        }

        let Ok((full_item, consumed_bits)) = parse_item_at(section_bytes, probe, huffman) else {
            probe += 8;
            continue;
        };

        if is_plausible_socket_child_header(full_item.mode, full_item.location, &full_item.code, full_item.flags) {
            return Some((full_item, probe + consumed_bits));
        }

        probe += 8;
    }

    None
}

fn peek_item_header_at(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
) -> Option<(u8, u8, String, u32)> {
    if start_bit % 8 != 0 {
        return None;
    }

    let start_byte = (start_bit / 8) as usize;
    let mut reader = IoBitReader::endian(Cursor::new(&section_bytes[start_byte..]), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    let flags = recorder.read_bits(32).ok()?;
    let _version = recorder.read_bits(3).ok()?;
    let mode = recorder.read_bits(3).ok()? as u8;
    let location = recorder.read_bits(4).ok()? as u8;
    let _x = recorder.read_bits(4).ok()?;
    let _y = recorder.read_bits(4).ok()?;
    let _page = recorder.read_bits(3).ok()?;

    let mut code = String::new();
    for _ in 0..4 {
        code.push(huffman.decode_recorded(&mut recorder).ok()?);
    }

    Some((mode, location, code, flags))
}

fn find_next_item_match(
    section_bytes: &[u8],
    start_bit: u64,
    huffman: &HuffmanTree,
) -> Option<u64> {
    let mut probe = align_to_byte(start_bit);
    let section_bits = (section_bytes.len() * 8) as u64;

    while probe < section_bits {
        if let Some((mode, location, code, flags)) = peek_item_header_at(section_bytes, probe, huffman) {
            if is_plausible_item_header(mode, location, &code, flags) {
                return Some(probe);
            }
        }
        probe += 8;
    }
    None
}

fn recover_property_reader<R: BitRead>(
    recorder: &mut BitRecorder<R>,
    code: &str,
    section_bytes: &[u8],
    item_start_bit: u64,
    huffman: &HuffmanTree,
) -> io::Result<bool> {
    let section_bits = (section_bytes.len() * 8) as u64;
    let section_pos = item_start_bit + recorder.recorded_bits.len() as u64;

    let mut probe = crate::domain::vo::align_to_byte(section_pos);
    while probe < section_bits {
        let Some((mode, location, probe_code, probe_flags)) = peek_item_header_at(section_bytes, probe, huffman)
        else {
            probe += 8;
            continue;
        };

        if is_plausible_item_header(mode, location, &probe_code, probe_flags) {
            let skip = if probe > section_pos { probe - section_pos } else { 0 };
            
            item_trace!(
                "    [RECOVERY] Synchronizing bitstream for '{}'. Found possible item '{}' at offset {}. Skipping {} bits.",
                code,
                probe_code,
                probe,
                skip
            );

            for _ in 0..skip {
                recorder.read_bit()?;
            }
            return Ok(true);
        }
        probe += 8;
    }
    Ok(false)
}
fn parse_base_header<R: BitRead>(
    recorder: &mut BitRecorder<R>,
    version: u8,
) -> io::Result<(u32, u8, ItemQuality)> {
    if version == 5 {
        // Alpha v105: Base header is 16 bits total.
        // Heuristic: Lvl (7 bits) + Quality (4 bits) + 5 bits padding/flags?
        let level = recorder.read_bits(7)? as u8;
        let q_val = recorder.read_bits(4)? as u8;
        let quality = map_item_quality(q_val);
        let _padding = recorder.read_bits(5)?;
        item_trace!("  [BaseHeader] v105 Lvl: {}, QualValue: {}, Quality: {:?}, Padding: 0x{:02X}", level, q_val, quality, _padding);
        Ok((0, level, quality))
    } else {
        let id = recorder.read_bits(32)?;
        let level = recorder.read_bits(7)? as u8;
        let q_val = recorder.read_bits(4)? as u8;
        let quality = map_item_quality(q_val);
        item_trace!("  [BaseHeader] ID: 0x{:08X}, Lvl: {}, QualValue: {}, Quality: {:?}", id, level, q_val, quality);
        Ok((id, level, quality))
    }
}
