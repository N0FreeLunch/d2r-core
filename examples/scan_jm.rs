use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let fixture_path = "tests/fixtures/savegames/original/amazon_10_scrolls.d2s";
    let bytes = fs::read(fixture_path).expect("Fixture not found");
    
    let jm_pos = (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .expect("JM header not found");
    
    let section_bytes = &bytes[jm_pos + 4..];
    let limit = (section_bytes.len() * 8) as u64;
    
    println!("Scanning for JM marker (0x4D4A) 1-bit granular:");
    for bit_offset in 0..(limit - 16) {
        let mut reader = BitReader::endian(Cursor::new(section_bytes), LittleEndian);
        let _ = reader.skip(bit_offset as u32).unwrap();
        let val = reader.read::<16, u16>().unwrap();
        if val == 0x4D4A {
             println!("Found JM marker at bit {}", bit_offset);
        }
    }
}
