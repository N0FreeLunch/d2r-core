use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: scan_terminator <save_file> <offset_bits>");
        return;
    }
    let bytes = fs::read(&args[1]).expect("failed to read save file");
    let start_bit = args[2].parse::<u64>().expect("invalid offset");

    println!("Scanning for property terminator (0x1FF) around bit {}:", start_bit);
    
    for nudge in -32i64..=128i64 {
        let current = (start_bit as i64 + nudge) as u64;
        if current >= (bytes.len() * 8) as u64 { continue; }
        
        let mut reader = IoBitReader::endian(Cursor::new(&bytes[(current / 8) as usize..]), LittleEndian);
        let mut recorder = BitCursor::new(&mut reader);
        let _ = recorder.skip_and_record((current % 8) as u32);
        
        let id: u32 = recorder.read_bits::<u32>(9).unwrap_or(0);
        if id == 0x1FF {
            println!("  [Bit {}] Terminator found! (nudge {})", current, nudge);
        }
    }
}
