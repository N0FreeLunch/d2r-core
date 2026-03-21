use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();
    let start_bit = 8349;

    println!("Scanning for 0x1FF (9 ones) in Item 14...");
    for offset in 0..300 {
        let mut temp_reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let _ = temp_reader.skip((start_bit + offset) as u32);
        if let Ok(val) = temp_reader.read::<9, u32>() {
            if val == 0x1FF {
                println!(
                    "  - Found at offset {} (bit {})",
                    offset,
                    start_bit + offset
                );
            }
        }
    }
}
