use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: d2item_bit_dump <save_file> <base_bit> [rows=16] [width=9]");
        process::exit(1);
    }

    let path = &args[1];
    let base_bit: usize = args[2].parse().expect("base_bit must be a number");
    let rows: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
    let width: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(9);

    let bytes = fs::read(path).expect("failed to read save file");
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    
    if reader.skip(base_bit as u32).is_err() {
        eprintln!("Error: Cannot skip to bit {} (outside file boundaries).", base_bit);
        process::exit(1);
    }

    println!("Dumping raw bits from {}...", base_bit);
    println!("Visual Matrix: {} rows x {} width", rows, width);
    println!("------------------------------------------------------------");

    for r in 0..rows {
        let mut row_str = String::new();
        for _ in 0..width {
            match reader.read_bit() {
                Ok(bit) => row_str.push(if bit { '1' } else { '0' }),
                Err(_) => break,
            }
        }
        if row_str.is_empty() {
             break;
        }
        let current_pos = base_bit + (r + 1) * width;
        println!("{} (pos={})", row_str, current_pos);
    }
    println!("------------------------------------------------------------");
}
