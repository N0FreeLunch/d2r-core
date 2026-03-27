use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_10_scrolls.d2s").unwrap();
    // 'jav ' huffman: 000101110 (j) 11110 (a) 1101110 (v) 10 ( )
    let patterns = vec![
        (
            "jav ",
            vec![
                false, false, false, true, false, true, true, true, false, true, true, true, true,
                false, true, true, false, true, true, true, false, true, false,
            ],
        ),
        (
            "plt ",
            vec![
                true, true, false, false, true, true, true, true, false, true, false, true, true,
                false, false, true, false,
            ],
        ),
        (
            "buc ",
            vec![
                false, true, false, true, false, false, false, false, true, true, false, true,
                true, false, false, true, false,
            ],
        ),
    ];

    for (name, pattern) in patterns {
        println!("Searching for '{}'...", name);
        for i in 8000..10000 {
            let mut reader = BitReader::endian(io::Cursor::new(&bytes), LittleEndian);
            if reader.skip(i).is_err() {
                break;
            }
            let mut match_found = true;
            for &expected in &pattern {
                if let Ok(b) = reader.read_bit() {
                    if b != expected {
                        match_found = false;
                        break;
                    }
                } else {
                    match_found = false;
                    break;
                }
            }
            if match_found {
                println!("  Found '{}' at Global Bit {}", name, i);
            }
        }
    }
}
