use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use std::io::{self, Cursor};

pub struct HuffmanTree {
    root: Box<HuffmanNode>,
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
    reader: &'a mut R,
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

        for (symbol, pattern) in table {
            let mut current = &mut root;
            for bit in pattern.chars() {
                if bit == '1' {
                    if current.right.is_none() {
                        current.right = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.right.as_mut().unwrap();
                } else {
                    if current.left.is_none() {
                        current.left = Some(Box::new(HuffmanNode::new()));
                    }
                    current = current.left.as_mut().unwrap();
                }
            }
            current.symbol = Some(symbol);
        }
        HuffmanTree { root }
    }

    pub fn decode_recorded<R: BitRead>(&self, recorder: &mut BitRecorder<R>) -> io::Result<char> {
        let mut current = &self.root;
        loop {
            if let Some(s) = current.symbol {
                return Ok(s);
            }
            let bit = recorder.read_bit()?;
            current = if bit {
                current.right.as_ref().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit")
                })?
            } else {
                current.left.as_ref().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit")
                })?
            };
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
                current.right.as_ref().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit")
                })?
            } else {
                current.left.as_ref().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid Huffman bit")
                })?
            };
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
        let cs_bytes = cs.to_le_bytes();
        bytes[12..16].copy_from_slice(&cs_bytes);
    }
}

#[derive(Debug, Clone)]
pub struct Item {
    pub bits: Vec<bool>,
    pub code: String,
    pub x: u8,
    pub y: u8,
    pub page: u8,
}

impl Item {
    pub fn set_bits(bits: &mut [bool], pos: usize, val: u32, count: u32) {
        for i in 0..count {
            bits[pos + i as usize] = (val >> i) & 1 != 0;
        }
    }

    pub fn from_reader<R: BitRead>(reader: &mut R, huffman: &HuffmanTree) -> io::Result<Self> {
        let mut recorder = BitRecorder::new(reader);
        let flags = recorder.read_bits(32)?;
        let _version = recorder.read_bits(3)?;
        let _mode = recorder.read_bits(3)?;
        let _loc = recorder.read_bits(4)?;
        let x = (recorder.read_bits(4)? & 0x0F) as u8;
        let y = (recorder.read_bits(4)? & 0x0F) as u8;
        let page = (recorder.read_bits(3)? & 0x07) as u8;

        let mut code = String::new();
        for _ in 0..4 {
            code.push(huffman.decode_recorded(&mut recorder)?);
        }

        let is_compact = (flags & (1 << 21)) != 0;
        let num_socket_bits = if is_compact { 1 } else { 3 };
        let _ = recorder.read_bits(num_socket_bits)?;

        Ok(Item {
            bits: recorder.recorded_bits,
            code,
            x,
            y,
            page,
        })
    }

    pub fn scan_items(bytes: &[u8], huffman: &HuffmanTree) -> Vec<(usize, String)> {
        let start_scan = 905 * 8;
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
                    "hp1 ", "mp1 ", "tsc ", "isc ", "buc ", "jav ", "wwa7", "vps ", "aqv ", "key ",
                    "tbk ", "ibk ",
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
