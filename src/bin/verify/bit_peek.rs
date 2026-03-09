use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
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

    let jm_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .unwrap();
    let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
    println!("JM at {}, count {}", jm_pos, count);

    let huffman = HuffmanTree::new();
    let mut reader = BitReader::endian(Cursor::new(&bytes[jm_pos + 4..]), LittleEndian);

    for i in 0..count {
        let bit_start = (jm_pos + 4) * 8 + reader.position_in_bits().unwrap() as usize;
        match Item::from_reader(&mut reader, &huffman) {
            Ok(item) => {
                println!(
                    "Item {}: '{}' (start={}, len={})",
                    i,
                    item.code,
                    bit_start,
                    item.bits.len()
                );
                if i == 0 {
                    // Peek at next 64 bits binary
                    let next = reader.peek_bits(64).unwrap_or(0);
                    println!("Next 64 bits: {:064b}", next);
                }
            }
            Err(e) => {
                println!("Error at {}: {}", i, e);
                break;
            }
        }
    }
}
