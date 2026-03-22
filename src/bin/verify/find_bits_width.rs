use d2r_core::item::{BitRecorder, HuffmanTree};
use bitstream_io::{BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let bit_pos = 7790; // Start of List 1
    
    for val_bits in 1..20 {
        let byte_offset = bit_pos / 8;
        let bit_offset = bit_pos % 8;
        
        let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
        let mut recorder = BitRecorder::new(&mut reader);
        for _ in 0..bit_offset { recorder.read_bit().ok(); }
        
        let id = recorder.read_bits(9).unwrap();
        if id != 16 { continue; }
        
        let val = recorder.read_bits(val_bits).unwrap();
        
        // Try reading next ID (9 bits)
        let next_id = recorder.read_bits(9).unwrap_or(0);
        if next_id == 8 { // Expected next ID for Authority
            println!("SUCCESS! ID 16 value bits = {}, value = {}, Next ID = 8", val_bits, val);
        }
    }
}
