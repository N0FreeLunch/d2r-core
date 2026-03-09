use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).unwrap();
    let huffman = HuffmanTree::new();

    // Find first JM
    let jm_pos = (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .unwrap();
    let item_count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
    println!("Found JM at byte {}, item count: {}", jm_pos, item_count);

    let start_bit = (jm_pos + 4) * 8;
    let end_bit = bytes.len() * 8 - 40;

    println!("Scanning all bit offsets for known codes...");
    for start in start_bit..end_bit {
        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let _ = reader.skip(start as u32);

        let mut code = String::new();
        let mut valid = true;
        for _ in 0..4 {
            match huffman.decode(&mut reader) {
                Ok(c) => code.push(c),
                Err(_) => {
                    valid = false;
                    break;
                }
            }
        }

        if valid {
            let expected = ["hp1 ", "mp1 ", "tsc ", "isc ", "jav ", "buc "];
            if expected.contains(&code.as_str()) {
                println!(
                    "  - Bit {:>5} (byte {:>4}, bit {}): code '{}'",
                    start,
                    start / 8,
                    start % 8,
                    code
                );
            }
        }
    }
}
