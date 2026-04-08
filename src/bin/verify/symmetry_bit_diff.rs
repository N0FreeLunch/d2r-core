use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::{HuffmanTree, Item, RecordedBit};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: symmetry_bit_diff <save_file_a> <save_file_b>");
        return;
    }
    let bytes_a = fs::read(&args[1]).expect("failed to read save file A");
    let bytes_b = fs::read(&args[2]).expect("failed to read save file B");

    let jm_pos_a = find_jm(&bytes_a).expect("No JM in A");
    let jm_pos_b = find_jm(&bytes_b).expect("No JM in B");

    let huffman = HuffmanTree::new();
    let is_alpha_a = bytes_a[4..8] == [0x69, 0, 0, 0];
    let is_alpha_b = bytes_b[4..8] == [0x69, 0, 0, 0];

    let mut reader_a = IoBitReader::endian(Cursor::new(&bytes_a[jm_pos_a + 4..]), LittleEndian);
    let mut reader_b = IoBitReader::endian(Cursor::new(&bytes_b[jm_pos_b + 4..]), LittleEndian);
    
    let mut cursor_a = BitCursor::new(&mut reader_a);
    let mut cursor_b = BitCursor::new(&mut reader_b);

    println!("Comparing bitstream symmetry between A and B...");

    loop {
        let item_a = Item::from_reader_with_context(&mut cursor_a, &huffman, Some((&bytes_a, ((jm_pos_a + 4) * 8) as u64)), is_alpha_a);
        let item_b = Item::from_reader_with_context(&mut cursor_b, &huffman, Some((&bytes_b, ((jm_pos_b + 4) * 8) as u64)), is_alpha_b);

        match (item_a, item_b) {
            (Ok(ia), Ok(ib)) => {
                println!("Item match: '{}' (A len={} bits, B len={} bits)", ia.code, ia.bits.len(), ib.bits.len());
                if ia.bits.len() != ib.bits.len() {
                    println!("  [DIFF] Bit length mismatch!");
                    // Forensic bit-by-bit diff could go here
                }
            }
            (Err(ea), Err(eb)) => {
                println!("Both failed: A={}, B={}", ea, eb);
                break;
            }
            (Ok(ia), Err(eb)) => {
                println!("Mismatch: A succeeded ('{}'), B failed: {}", ia.code, eb);
                break;
            }
            (Err(ea), Ok(ib)) => {
                println!("Mismatch: A failed ({}), B succeeded ('{}')", ea, ib.code);
                break;
            }
        }
    }
}

fn find_jm(bytes: &[u8]) -> Option<usize> {
    (0..bytes.len() - 2).find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
}
