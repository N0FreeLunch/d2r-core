use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, BitRecorder};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        eprintln!("Usage: d2item_bit_search <save_file> <base_bit> <pattern_length_bits> <pattern_value> [window_bits]");
        eprintln!("Example: d2item_bit_search char.d2s 8349 9 511 300");
        process::exit(1);
    }

    let path = &args[1];
    let base_bit: usize = args[2].parse().expect("base_bit must be a number");
    let bits: u32 = args[3].parse().expect("pattern_length_bits must be a number");
    let value: u32 = args[4].parse().expect("pattern_value must be a number");
    let window: usize = args.get(5).and_then(|s| s.parse().ok()).unwrap_or(300);

    let bytes = fs::read(path).expect("failed to read save file");

    println!("Scanning for pattern (Length: {} bits, Value: {}) starting from {}...", bits, value, base_bit);
    println!("Scan Window: {} bits", window);

    let mut found = 0;
    for offset in 0..=window {
        let mut reader = IoBitReader::endian(Cursor::new(&bytes), LittleEndian);
        let target = base_bit + offset;
        
        if reader.skip(target as u32).is_err() {
             break;
        }
        
        let mut recorder = BitRecorder::new(&mut reader);
        match recorder.read_bits(bits) {
            Ok(v) if v == value => {
                println!("  [FOUND] Value {} at Bit Offset {} (Distance: {} bits from base)", value, target, offset);
                found += 1;
            }
            _ => {}
        }
    }

    if found == 0 {
        println!("No matching pattern found in the specified window.");
    } else {
        println!("Scan complete. Found {} occurrences.", found);
    }
}
