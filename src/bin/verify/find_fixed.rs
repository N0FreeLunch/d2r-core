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
    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();

    println!("--- Testing Variable Fixed (Total 24bits) ---");

    for id_bits in 7..=11 {
        let v_bits = 24 - id_bits;
        for start_bit in 7780..=7800 {
            let byte_offset = start_bit / 8;
            let bit_offset = start_bit % 8;
            let mut reader =
                BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
            for _ in 0..bit_offset {
                let _ = reader.read_bit().ok();
            }

            for _ in 0..10 {
                let _id = read_bits(&mut reader, id_bits);
                let val = read_bits(&mut reader, v_bits);
                if val == 14 || val == 310 || val == 31 {
                    println!(
                        "HIT! ID: {}, Val bits: {}, Start: {}, Value: {}",
                        id_bits, v_bits, start_bit, val
                    );
                }
            }
        }
    }
}
