use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();

    let jav_start = 8296;
    let buc_start = 8517; // This may be wrong if jav length is wrong

    println!("Scanning 0x1FF for Item 14 (jav start {})...", jav_start);
    for len in 50..250 {
        if check(jav_start + len, &bytes) {
            println!("  - Offset {} (bit {})", len, jav_start + len);
        }
    }
}

fn check(start: usize, bytes: &[u8]) -> bool {
    let mut reader = BitReader::endian(Cursor::new(bytes), LittleEndian);
    if reader.skip(start as u32).is_err() {
        return false;
    }

    // Read 0x1FF
    match reader.read::<9, u32>() {
        Ok(511) => true,
        _ => false,
    }
}
