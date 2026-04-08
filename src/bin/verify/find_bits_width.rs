use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: find_bits_width <save_file> <offset_bits> [count]");
        return;
    }
    let bytes = fs::read(&args[1]).expect("failed to read save file");
    let offset = args[2].parse::<u64>().expect("invalid offset");
    let count = args.get(3).and_then(|s| s.parse::<usize>().ok()).unwrap_or(10);

    let mut reader = IoBitReader::endian(Cursor::new(&bytes[(offset / 8) as usize..]), LittleEndian);
    let mut recorder = BitCursor::new(&mut reader);
    let _ = recorder.skip_and_record((offset % 8) as u32);

    println!("Probing bit widths at offset {}:", offset);
    for width in 1..=32 {
        let checkpoint = recorder.checkpoint();
        let val: u32 = recorder.read_bits::<u32>(width as u32).unwrap_or(0);
        println!("  Width {:2}: {} (0x{:X})", width, val, val);
        recorder.rollback(checkpoint);
    }
}
