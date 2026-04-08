use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: find_list1 <save_file> <item_start_bit>");
        return;
    }
    let bytes = fs::read(&args[1]).expect("failed to read save file");
    let start_bit = args[2].parse::<u64>().expect("invalid start bit");

    let mut reader = IoBitReader::endian(Cursor::new(&bytes[(start_bit / 8) as usize..]), LittleEndian);
    let mut recorder = BitCursor::new(&mut reader);
    let _ = recorder.skip_and_record((start_bit % 8) as u32);

    println!("Scanning for List 1 properties (9-bit IDs) after header/code at {}:", start_bit);
    // Assume header + code is roughly 100-150 bits
    for skip in (80..200).step_by(1) {
        let checkpoint = recorder.checkpoint();
        let _ = recorder.skip_and_record(skip as u32);
        
        let id: u32 = recorder.read_bits::<u32>(9).unwrap_or(0);
        if id < 511 && id > 0 {
             println!("  [Skip {}] Potential Stat ID: {} at bit {}", skip, id, start_bit + skip as u64);
        }
        recorder.rollback(checkpoint);
    }
}
