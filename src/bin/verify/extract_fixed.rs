use std::env;
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
    let args: Vec<String> = env::args().collect();
    let start_bit: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(7790);
    let total_width: u32 = args.get(2).and_then(|w| w.parse().ok()).unwrap_or(24);
    let id_bits = 9u32;
    let val_bits = total_width - id_bits;

    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    
    println!("--- Alpha v105 Fixed {}-bit Property Extraction ({} ID + {} VAL) ---", total_width, id_bits, val_bits);

    let byte_offset = start_bit / 8;
    let bit_offset = start_bit % 8;
    let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
    for _ in 0..bit_offset { let _ = reader.read_bit().ok(); }
    
    for i in 0..20 {
        let pos = start_bit + (i as u64) * (total_width as u64);
        let id = read_bits(&mut reader, id_bits);
        let val = read_bits(&mut reader, val_bits);
        println!("Prop {:>2}: ID={:<3}, Val={:<8} (Bit={})", i, id, val, pos);
        if id == 511 { break; }
    }
}
