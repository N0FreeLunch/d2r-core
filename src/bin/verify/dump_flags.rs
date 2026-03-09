use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();
    let jm_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .unwrap();
    let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);

    let mut reader = BitReader::endian(Cursor::new(&bytes[jm_pos + 4..]), LittleEndian);
    for i in 0..count {
        let _ = reader.byte_align();
        let pos = reader.position_in_bits().unwrap_or(0);
        let bit_start = (jm_pos + 4) * 8 + pos as usize;

        println!("Item {} (bit {}):", i, bit_start);
        let mut temp = reader.clone();
        for _ in 0..4 {
            let val = temp.read::<32, u32>().unwrap_or(0);
            println!("  0x{:08X} ({:032b})", val, val);
        }

        // Skip ahead to next item based on previous findings (approx)
        if i < 14 {
            let _ = reader.skip(70);
        } else {
            let _ = reader.skip(160);
        }
    }
}
