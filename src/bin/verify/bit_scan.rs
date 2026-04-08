use bitstream_io::{BitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: bit_scan <save_file>");
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).unwrap();
    let huffman = HuffmanTree::new();

    let mut starts = Vec::new();
    let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];

    // Simple brute force bit scan
    for bit_pos in 0..(bytes.len() as u64 * 8 - 100) {
        let b_start = (bit_pos / 8) as usize;
        let b_off = (bit_pos % 8) as u32;
        
        let cursor = Cursor::new(&bytes[b_start..]);
        let reader = BitReader::endian(cursor, LittleEndian);
        let mut recorder = BitCursor::new(reader);
        if b_off > 0 {
            let _ = recorder.skip_and_record(b_off).ok();
        }

        if let Ok(item) = Item::from_reader_with_context(&mut recorder, &huffman, Some((&bytes, bit_pos)), is_alpha) {
            if recorder.pos() >= 32 {
                starts.push((bit_pos, item.code.clone()));
            }
        }
    }

    println!("Found {} items via scan:", starts.len());
    for (bit, code) in starts {
        println!("  Bit {}: code '{}'", bit, code);
    }
}
