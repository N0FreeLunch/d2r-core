use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();
    let start_bit = 8349 + 23 + 32 + 7 + 3 + 3; // After Huffman, ID, level, Qual, Graphic
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    let _ = reader.skip(start_bit as u32);

    for i in 0..256 {
        let bit = reader.read_bit().unwrap();
        print!("{}", if bit { "1" } else { "0" });
        if (i + 1) % 9 == 0 {
            println!(" (bit={})", start_bit + i + 1);
        }
    }
}
