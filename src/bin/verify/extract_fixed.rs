use d2r_core::item::HuffmanTree;
use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn read_bits<R: BitRead>(reader: &mut R, n: u32) -> u32 {
    let mut value = 0u32;
    for i in 0..n {
        if let Ok(b) = reader.read_bit() {
            if b {
                value |= 1 << i;
            }
        }
    }
    value
}

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let start_bit = 7790;
    
    println!("--- Alpha v105 Fixed 24-bit Property Extraction (9 ID + 15 VAL) ---");

    let byte_offset = start_bit / 8;
    let bit_offset = start_bit % 8;
    let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
    for _ in 0..bit_offset { let _ = reader.read_bit().ok(); }
    
    for i in 0..20 {
        let id = read_bits(&mut reader, 9);
        let val = read_bits(&mut reader, 15);
        println!("Prop {:>2}: ID={:<3}, Val={:<5} (Bit={})", i, id, val, start_bit + i*24);
        if id == 511 { break; }
    }
}
