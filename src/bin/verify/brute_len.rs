use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::item::BitRecorder;
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: d2item_brute_len <save_file> <base_bit> [min_len] [max_len]");
        process::exit(1);
    }

    let path = &args[1];
    let base_bit: usize = args[2].parse().expect("base_bit must be a number");
    let min_len: usize = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(50);
    let max_len: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(300);

    let bytes = fs::read(path).expect("failed to read save file");

    println!(
        "Scanning 9-bit Terminator (511) starting from {}...",
        base_bit
    );
    println!("Range: {} to {} bits from base", min_len, max_len);

    let mut found = 0;
    for len in min_len..=max_len {
        let target = base_bit + len;
        if check(target, &bytes) {
            println!(
                "  [MATCH] Found 511 at Bit Offset {} (Length: {} bits from base)",
                target, len
            );
            found += 1;
        }
    }

    if found == 0 {
        println!("No terminator found in the specified range.");
    } else {
        println!("Scan complete. Found {} potential terminators.", found);
    }
}

fn check(start: usize, bytes: &[u8]) -> bool {
    let mut reader = IoBitReader::endian(Cursor::new(bytes), LittleEndian);
    if reader.skip(start as u32).is_err() {
        return false;
    }

    let mut recorder = BitRecorder::new(&mut reader);
    match recorder.read_bits(9) {
        Ok(511) => true,
        _ => false,
    }
}
