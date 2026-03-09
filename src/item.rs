use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use std::io::{self, Cursor};

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

pub struct BitRecorder<'a, R: BitRead> {
    pub reader: &'a mut R,
    pub recorded_bits: Vec<bool>,
}

impl<'a, R: BitRead> BitRecorder<'a, R> {
    pub fn new(reader: &'a mut R) -> Self {
        BitRecorder {
            reader,
            recorded_bits: Vec::new(),
        }
    }

    pub fn read_bit(&mut self) -> io::Result<bool> {
        let bit = self.reader.read_bit()?;
        self.recorded_bits.push(bit);
        Ok(bit)
    }

    pub fn read_bits(&mut self, count: u32) -> io::Result<u32> {
        let mut val = 0;
        for i in 0..count {
            if self.read_bit()? {
                val |= 1 << i;
            }
        }
        Ok(val)
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

    pub fn decode_recorded<R: BitRead>(&self, recorder: &mut BitRecorder<R>) -> io::Result<char> {
        let mut current = &self.root;
        loop {
            if let Some(s) = current.symbol {
                return Ok(s);
            }
            let bit = recorder.read_bit()?;
            current = if bit {
                current.right.as_ref()
            } else {
                current.left.as_ref()
            }
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit"))?;
        }
    }

    pub fn decode<R: BitRead>(&self, reader: &mut R) -> io::Result<char> {
        let mut current = &self.root;
        loop {
            if let Some(s) = current.symbol {
                return Ok(s);
            }
            let bit = reader.read_bit()?;
            current = if bit {
                current.right.as_ref()
            } else {
                current.left.as_ref()
            }
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit"))?;
        }
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

#[derive(Debug, Clone)]
pub struct Item {
    pub bits: Vec<bool>,
    pub code: String,
    pub x: u8,
    pub y: u8,
    pub page: u8,
    pub location: u8,
    pub id: Option<u32>,
    pub level: Option<u8>,
    pub quality: Option<u8>,
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
    pub rare_affixes: Vec<u16>,
    pub unique_id: Option<u16>,
    pub properties: Vec<ItemProperty>,
}

#[derive(Debug, Clone)]
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

fn read_property_list<R: BitRead>(
    recorder: &mut BitRecorder<R>,
    code: &str,
) -> io::Result<Vec<ItemProperty>> {
    let mut props = Vec::new();

    loop {
        let bit_pos = recorder.recorded_bits.len();
        let stat_id = recorder.read_bits(9)?;
        if stat_id == 0x1FF {
            return Ok(props);
        }

        let cost = crate::data::stat_costs::STAT_COSTS
            .iter()
            .find(|s| s.id == stat_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Unknown stat ID {} at bit {} (code='{}')",
                        stat_id, bit_pos, code
                    ),
                )
            })?;

        let param = if cost.save_param_bits > 0 {
            recorder.read_bits(cost.save_param_bits as u32)?
        } else {
            0
        };

        let raw_value = if cost.save_bits > 0 {
            recorder.read_bits(cost.save_bits as u32)? as i32
        } else {
            0
        };

        let value = raw_value - cost.save_add;

        props.push(ItemProperty {
            stat_id,
            name: cost.name.to_string(),
            param,
            raw_value,
            value,
        });
    }
}

impl Item {
    pub fn from_reader<R: BitRead>(reader: &mut R, huffman: &HuffmanTree) -> io::Result<Self> {
        let mut recorder = BitRecorder::new(reader);

        let flags = recorder.read_bits(32)?;
        let _version = recorder.read_bits(3)?;
        let _mode = recorder.read_bits(3)?;
        let loc = recorder.read_bits(4)? as u8;
        let x = (recorder.read_bits(4)? & 0x0F) as u8;
        let y = (recorder.read_bits(4)? & 0x0F) as u8;
        let page = (recorder.read_bits(3)? & 0x07) as u8;

        let is_identified = (flags & (1 << 4)) != 0;
        let is_socketed = (flags & (1 << 11)) != 0;
        let is_ear = (flags & (1 << 16)) != 0;
        let is_compact = (flags & (1 << 21)) != 0;
        let is_ethereal = (flags & (1 << 22)) != 0;
        let is_personalized = (flags & (1 << 24)) != 0;
        let is_runeword = (flags & (1 << 26)) != 0;

        let code = if is_ear {
            let _ = recorder.read_bits(3)?; // ear class
            let _ = recorder.read_bits(7)?; // ear level
            let _ = read_player_name(&mut recorder)?;
            "ear ".to_string()
        } else {
            let mut decoded = String::new();
            for _ in 0..4 {
                decoded.push(huffman.decode_recorded(&mut recorder)?);
            }
            decoded
        };

        if is_ear {
            return Ok(Item {
                bits: recorder.recorded_bits,
                code,
                x,
                y,
                page,
                location: loc,
                id: None,
                level: None,
                quality: None,
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
                rare_affixes: Vec::new(),
                unique_id: None,
                properties: Vec::new(),
            });
        }

        let trimmed_code = code.trim();
        println!("  [Item] Code: '{}', Flags: 0x{:08X}", trimmed_code, flags);

        // D2R: Every item seems to have 3 bits here (potentially num sockets, but present even if not socketed)
        let _num_sockets_d2r = recorder.read_bits(3)?;

        let mut item_id = None;
        let mut item_level = None;
        let mut item_quality = None;
        let mut magic_prefix = None;
        let mut magic_suffix = None;
        let mut rare_name_1 = None;
        let mut rare_name_2 = None;
        let mut rare_affixes = Vec::new();
        let mut unique_id = None;

        let mut properties = Vec::new();

        if !is_compact {
            let template = item_template(&code);
            item_id = Some(recorder.read_bits(32)?);
            item_level = Some(recorder.read_bits(7)? as u8);
            let quality = recorder.read_bits(4)? as u8;
            item_quality = Some(quality);
            println!(
                "  [Stats] ID: {:?}, Lvl: {:?}, Quality: {}",
                item_id, item_level, quality
            );

            let has_multiple_graphics = recorder.read_bits(1)? != 0;
            if has_multiple_graphics {
                let _ = recorder.read_bits(3)?;
            }
            let has_class_specific_data = recorder.read_bits(1)? != 0;
            if has_class_specific_data {
                let _ = recorder.read_bits(11)?;
            }

            match quality {
                1 | 3 => {
                    // Low or High Quality
                    let _ = recorder.read_bits(3)?;
                }
                4 => {
                    // Magic
                    magic_prefix = Some(recorder.read_bits(11)? as u16);
                    magic_suffix = Some(recorder.read_bits(11)? as u16);
                }
                6 | 8 => {
                    // Rare or Crafted
                    rare_name_1 = Some(recorder.read_bits(8)? as u8);
                    rare_name_2 = Some(recorder.read_bits(8)? as u8);
                    for _ in 0..3 {
                        if recorder.read_bits(1)? != 0 {
                            rare_affixes.push(recorder.read_bits(11)? as u16);
                        }
                        if recorder.read_bits(1)? != 0 {
                            rare_affixes.push(recorder.read_bits(11)? as u16);
                        }
                    }
                }
                5 | 7 => {
                    // Set or Unique
                    unique_id = Some(recorder.read_bits(12)? as u16);
                }
                _ => {}
            }

            if is_runeword {
                let _ = recorder.read_bits(12)?;
                let _ = recorder.read_bits(4)?;
            }

            if is_personalized {
                let _ = read_player_name(&mut recorder)?;
            }

            if trimmed_code == "tbk" || trimmed_code == "ibk" {
                let _ = recorder.read_bits(5)?;
            }

            let has_realm_data = recorder.read_bits(1)? != 0;
            if has_realm_data {
                let _ = recorder.read_bits(32)?;
                let _ = recorder.read_bits(32)?;
                let _ = recorder.read_bits(32)?;
            }

            if let Some(template) = template {
                if template.is_armor {
                    let _def = recorder.read_bits(11)?;
                }
                if template.has_durability {
                    let max_dur = recorder.read_bits(8)?;
                    if max_dur > 0 {
                        let _cur_dur = recorder.read_bits(9)?;
                        let _dur_extra = recorder.read_bits(1)?;
                        println!(
                            "  [Dur] Max: {}, Cur: {} (+{})",
                            max_dur, _cur_dur, _dur_extra
                        );
                    }
                }
                if template.is_stackable {
                    let _qty = recorder.read_bits(9)?;
                    println!("  [Qty] {}", _qty);
                }
            }

            if is_socketed {
                let _sockets = recorder.read_bits(4)?;
            }

            properties = read_property_list(&mut recorder, trimmed_code)?;

            for p in &properties {
                println!(
                    "  [Prop] {}: param={} raw={} value={}",
                    p.name, p.param, p.raw_value, p.value
                );
            }
        }

        Ok(Item {
            bits: recorder.recorded_bits,
            code,
            x,
            y,
            page,
            location: loc,
            id: item_id,
            level: item_level,
            quality: item_quality,
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
            properties,
        })
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
}

#[cfg(test)]
mod tests {
    use super::{HuffmanTree, Item};
    use bitstream_io::{BitRead, BitReader, LittleEndian};
    use std::fs;
    use std::io::Cursor;

    #[test]
    fn parses_all_player_items_in_amazon_10_scrolls() {
        let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s")
            .expect("fixture should exist");
        let jm_pos = (0..bytes.len() - 2)
            .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
            .expect("fixture should contain a JM section");
        let item_count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
        let huffman = HuffmanTree::new();
        let mut reader = BitReader::endian(Cursor::new(&bytes[jm_pos + 4..]), LittleEndian);
        let mut codes = Vec::new();

        for _ in 0..item_count {
            let _ = reader.byte_align();
            let item = Item::from_reader(&mut reader, &huffman).expect("item should parse");
            codes.push(item.code);
        }

        assert_eq!(codes.len(), item_count as usize);
        assert_eq!(codes[14], "jav ");
        assert_eq!(codes[15], "buc ");
    }
}
