use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();
    let start_bit = 8296;
    let end_bit = 8517;
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    let _ = reader.skip(start_bit as u32);

    println!("Dumping bits from {} to {}:", start_bit, end_bit);
    for i in 0..(end_bit - start_bit) {
        let bit = reader.read_bit().unwrap();
        print!("{}", if bit { "1" } else { "0" });
        if (i + 1) % 8 == 0 {
            print!(" ");
        }
        if (i + 1) % 32 == 0 {
            println!(" (bit={})", start_bit + i + 1);
        }
    }
    println!();
}
