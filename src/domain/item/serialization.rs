use bitstream_io::{BitRead, BitWrite, BitWriter, LittleEndian};
use std::io;
use crate::domain::item::{Item, ItemProperty, RecordedBit, ItemBitRange};
use crate::domain::item::quality::ItemQuality;
use crate::domain::stats::lookup_alpha_map_by_effective;
use crate::domain::item::stat_list::stat_save_bits;
use crate::data::bit_cursor::BitCursor;
use crate::error::{ParsingResult, ParsingError};

pub struct BitEmitter {
    writer: BitWriter<Vec<u8>, LittleEndian>,
    written: u64,
}

impl BitEmitter {
    pub fn new() -> Self {
        BitEmitter {
            writer: BitWriter::endian(Vec::new(), LittleEndian),
            written: 0,
        }
    }

    pub fn write_bit(&mut self, bit: bool) -> io::Result<()> {
        self.writer.write_bit(bit)?;
        self.written += 1;
        Ok(())
    }

    pub fn write_bits(&mut self, value: u32, count: u32) -> io::Result<()> {
        for i in 0..count {
            let bit = (value >> i) & 1 != 0;
            self.write_bit(bit)?;
        }
        Ok(())
    }

    pub fn extend_bits<I>(&mut self, bits: I) -> io::Result<()>
    where
        I: IntoIterator<Item = bool>,
    {
        for bit in bits {
            self.write_bit(bit)?;
        }
        Ok(())
    }

    pub fn byte_align(&mut self) -> io::Result<()> {
        let padding = (8 - (self.written % 8)) % 8;
        self.writer.byte_align()?;
        self.written += padding;
        Ok(())
    }

    pub fn written_bits(&self) -> u64 {
        self.written
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.writer.into_writer()
    }
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
            if let Some(symbol) = current.symbol {
                return Ok(symbol);
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

    pub fn decode_recorded<R: BitRead>(&self, cursor: &mut BitCursor<R>) -> ParsingResult<char> {
        self.decode_internal(|| cursor.read_bit().map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{:?}", e))))
            .map_err(|_| {
                cursor.fail(ParsingError::InvalidHuffmanBit { bit_offset: cursor.pos() })
                    .with_hint("Huffman bit pattern does not match Alpha v105 table. Possible bit drift or misalignment.")
            })
    }

    pub fn decode<R: BitRead>(&self, reader: &mut R) -> io::Result<char> {
        self.decode_internal(|| reader.read_bit())
    }
}

impl Item {
    pub fn to_bytes(&self, huffman: &HuffmanTree, alpha_mode: bool) -> io::Result<Vec<u8>> {
        let mut emitter = BitEmitter::new();
        emitter.write_bits(self.flags, 32)?;
        emitter.write_bits(self.version as u32, 3)?;
        emitter.write_bits(self.mode as u32, 3)?;
        emitter.write_bits(self.location as u32, 3)?;
        emitter.write_bits(self.x as u32, 4)?;
        
        if !self.is_compact {
            emitter.write_bits(self.y as u32, 4)?;
            emitter.write_bits(self.page as u32, 3)?;
            emitter.write_bits(self.header_socket_hint as u32, 3)?;
        }

        if self.is_ear {
            emitter.write_bits(self.ear_class.unwrap_or(0) as u32, 3)?;
            emitter.write_bits(self.ear_level.unwrap_or(0) as u32, 7)?;
            write_player_name(&mut emitter, self.ear_player_name.as_deref().unwrap_or(""))?;
        } else {
            let encoded_code = huffman.encode(&self.code)?;
            emitter.extend_bits(encoded_code)?;
        }

        if !self.is_compact {
            if alpha_mode {
                emitter.write_bits(self.level.unwrap_or(0) as u32, 7)?;
                emitter.write_bits(self.quality.map(|q| q as u8).unwrap_or(0) as u32, 4)?;
                emitter.write_bits(0, 5)?; // Padding
                emitter.write_bit(self.has_multiple_graphics)?;
                emitter.write_bit(self.has_class_specific_data)?;
                emitter.write_bit(self.timestamp_flag)?;
            } else {
                emitter.write_bits(self.id.unwrap_or(0), 32)?;
                emitter.write_bits(self.level.unwrap_or(0) as u32, 7)?;
                emitter.write_bits(self.quality.map(|q| q as u8).unwrap_or(0) as u32, 4)?;
            }

            if self.has_multiple_graphics {
                emitter.write_bits(self.multi_graphics_bits.unwrap_or(0) as u32, 3)?;
            }
            if self.has_class_specific_data {
                let bits = if alpha_mode { 3 } else { 11 };
                emitter.write_bits(self.class_specific_bits.unwrap_or(0) as u32, bits)?;
            }

            let quality_val = self.quality.unwrap_or(ItemQuality::Normal);
            match quality_val {
                ItemQuality::Low | ItemQuality::High => {
                    emitter.write_bits(self.low_high_graphic_bits.unwrap_or(0) as u32, 3)?;
                }
                ItemQuality::Magic => {
                    let bits = if alpha_mode { 7 } else { 11 };
                    emitter.write_bits(self.magic_prefix.unwrap_or(0) as u32, bits)?;
                    emitter.write_bits(self.magic_suffix.unwrap_or(0) as u32, bits)?;
                }
                ItemQuality::Rare | ItemQuality::Crafted => {
                    emitter.write_bits(self.rare_name_1.unwrap_or(0) as u32, 8)?;
                    emitter.write_bits(self.rare_name_2.unwrap_or(0) as u32, 8)?;
                    for i in 0..6 {
                        if let Some(affix) = self.rare_affixes[i] {
                            emitter.write_bit(true)?;
                            emitter.write_bits(affix as u32, 11)?;
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

            if self.is_runeword && self.version != 5 {
                emitter.write_bits(self.runeword_id.unwrap_or(0) as u32, 12)?;
                emitter.write_bits(self.runeword_level.unwrap_or(0) as u32, 4)?;
            }

            if self.is_personalized {
                write_player_name(&mut emitter, self.personalized_player_name.as_deref().unwrap_or(""))?;
            }

            if self.code.trim() == "tbk" || self.code.trim() == "ibk" {
                emitter.write_bits(self.tbk_ibk_teleport.unwrap_or(0) as u32, 5)?;
            }

            emitter.write_bit(self.timestamp_flag)?;

            if let Some(template) = item_template(&self.code) {
                if template.is_armor {
                    let bits = stat_save_bits(31).unwrap_or(11);
                    emitter.write_bits(self.defense.unwrap_or(0), bits)?;
                }
                if template.has_durability {
                    let max_bits = stat_save_bits(73).unwrap_or(8);
                    let cur_bits = stat_save_bits(72).unwrap_or(9);
                    let m_dur = self.max_durability.unwrap_or(0);
                    emitter.write_bits(m_dur, max_bits)?;
                    if m_dur > 0 {
                        emitter.write_bits(self.current_durability.unwrap_or(0), cur_bits)?;
                        emitter.write_bit(false)?; // dur_extra
                    }
                }
                if template.is_stackable {
                    emitter.write_bits(self.quantity.unwrap_or(0), 9)?;
                }
            }

            if self.is_socketed {
                emitter.write_bits(self.sockets.unwrap_or(0) as u32, 4)?;
            }

            if quality_val == ItemQuality::Set {
                let set_list_val = match self.set_list_count {
                    1 => 1, 2 => 3, 3 => 7, 4 => 15, 5 => 31, _ => 0
                };
                emitter.write_bits(set_list_val, 5)?;
            }

            write_property_list(&mut emitter, &self.properties, self.version, false, self.terminator_bit, quality_val)?;
            for set_props in &self.set_attributes {
                write_property_list(&mut emitter, set_props, self.version, false, false, quality_val)?;
            }
            if self.is_runeword {
                write_property_list(&mut emitter, &self.runeword_attributes, self.version, true, false, quality_val)?;
            }
        }

        Ok(emitter.into_bytes())
    }

    pub fn serialize_section(items: &[Item], huffman: &HuffmanTree, alpha_mode: bool) -> io::Result<Vec<u8>> {
        let mut all_bytes = Vec::new();
        for item in items {
            all_bytes.extend(item.to_bytes(huffman, alpha_mode)?);
            for child in &item.socketed_items {
                all_bytes.extend(child.to_bytes(huffman, alpha_mode)?);
            }
        }
        Ok(all_bytes)
    }
}

fn write_player_name(emitter: &mut BitEmitter, name: &str) -> io::Result<()> {
    for ch in name.chars() {
        emitter.write_bits((ch as u8 & 0x7F) as u32, 7)?;
    }
    emitter.write_bits(0, 7)?;
    Ok(())
}

fn item_template(code: &str) -> Option<&'static crate::data::item_codes::ItemTemplate> {
    crate::data::item_codes::ITEM_TEMPLATES
        .iter()
        .find(|template| template.code == code.trim())
}

fn write_property_list(emitter: &mut BitEmitter, props: &[ItemProperty], version: u8, alpha_runeword: bool, terminator_bit: bool, quality: ItemQuality) -> io::Result<()> {
    if version == 5 && alpha_runeword {
        for _ in 0..93 {
            emitter.write_bit(false)?;
        }
    }
    let id_bits = 9;
    let terminator = (1 << id_bits) - 1;

    for prop in props {
        if version == 5 || version == 1 {
            let (raw_id, _save_bits) = if let Some(m) = lookup_alpha_map_by_effective(prop.stat_id) {
                (m.raw_id, 0)
            } else {
                (prop.stat_id, 0)
            };

            emitter.write_bits(raw_id, 9)?;
            if quality != ItemQuality::Normal {
                emitter.write_bits(prop.raw_value as u32, 1)?;
            }
        } else if version == 4 {
            emitter.write_bits(prop.stat_id, 9)?;
            emitter.write_bits(prop.raw_value as u32, 8)?;
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
    if version == 5 || version == 1 {
        emitter.write_bit(terminator_bit)?;
    }
    Ok(())
}
