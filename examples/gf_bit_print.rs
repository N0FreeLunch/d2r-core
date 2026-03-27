use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::save::map_core_sections;
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tmp/amazon_v105_unlocked.d2s").unwrap();
    let map = map_core_sections(&bytes).unwrap();
    let section_bytes = &bytes[map.gf_pos + 2..map.if_pos];

    let mut reader = BitReader::endian(Cursor::new(section_bytes), LittleEndian);

    println!("=== GF Raw Bits ({} bytes) ===", section_bytes.len());
    for b in section_bytes {
        print!("{:08b} ", b);
    }
    println!("\n");
}
