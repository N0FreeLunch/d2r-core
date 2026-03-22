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
    
    println!("--- Alpha v105 Property Brute Force (Start: {}) ---", start_bit);

    let targets = [310, 14, 31, 1];

    for id_bits in [7, 8, 9, 10] {
        for v_bits in 1..=20 {
            let byte_offset = start_bit / 8;
            let bit_offset = start_bit % 8;
            let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
            for _ in 0..bit_offset { let _ = reader.read_bit().ok(); }
            
            let mut found = Vec::new();
            let mut sequence = Vec::new();
            
            for _ in 0..20 {
                let id = read_bits(&mut reader, id_bits);
                if id == (1 << id_bits) - 1 { break; }
                let val = read_bits(&mut reader, v_bits);
                sequence.push((id, val));
                if targets.contains(&val) {
                    found.push(val);
                }
            }
            
            if found.len() >= 2 {
                println!("Hit! (ID: {}, Val: {}) Found: {:?} Sequence: {:?}", id_bits, v_bits, found, sequence);
            }
        }
    }
}
