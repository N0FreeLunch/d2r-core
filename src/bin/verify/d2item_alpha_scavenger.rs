use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;
use d2r_core::item::{HuffmanTree, BitRecorder, is_plausible_item_header};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: d2item_alpha_scavenger <save_path> [--start-bit <bit>] [--count <bits>] [--range <start> <end>]");
        return Ok(());
    }

    let path = &args[1];
    let bytes = fs::read(path)?;

    let mut start_bit = 0;
    let mut count = 0;
    let mut range_start = 0;
    let mut range_end = 0;

    let mut i = 2;
    while i < args.len() {
        if args[i] == "--start-bit" {
            start_bit = args[i+1].parse().unwrap_or(0);
            i += 1;
        } else if args[i] == "--count" {
            count = args[i+1].parse().unwrap_or(0);
            i += 1;
        } else if args[i] == "--range" {
            range_start = args[i+1].parse().unwrap_or(0);
            range_end = args[i+2].parse().unwrap_or(0);
            i += 2;
        }
        i += 1;
    }

    if count > 0 {
        println!("[AlphaScavenger] Dumping {} bits starting at {}...", count, start_bit);
        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        reader.skip(start_bit as u32)?;
        for b in 0..count {
            let bit = if reader.read_bit()? { "1" } else { "0" };
            print!("{}", bit);
            if (b + 1) % 8 == 0 { print!(" "); }
            if (b + 1) % 32 == 0 { println!(" (Offset {})", start_bit + b + 1); }
        }
        println!();
        return Ok(());
    }

    if range_end > 0 {
        println!("[AlphaScavenger] Scanning range {}..{}...", range_start, range_end);
        let huffman = HuffmanTree::new();
        let start_aligned = (range_start / 8) * 8;
        for bit_pos in (start_aligned..=range_end).step_by(8) {
            let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
            if reader.skip(bit_pos as u32).is_err() { continue; }
            let mut recorder = BitRecorder::new(&mut reader);

            let Ok(flags) = recorder.read_bits(32) else { continue };
            let Ok(version) = recorder.read_bits(3) else { continue };
            let version = version as u8;
            let Ok(mode) = recorder.read_bits(3) else { continue };
            let mode = mode as u8;
            let Ok(loc) = recorder.read_bits(4) else { continue };
            let loc = loc as u8;

            if !(version == 5 || version == 2 || version == 1 || version == 0) {
                continue;
            }

            for offset in 32..=64 {
                let mut h_reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
                if h_reader.skip((bit_pos + offset) as u32).is_err() { continue; }
                let mut h_recorder = BitRecorder::new(&mut h_reader);
                
                if loc < 4 { let _ = h_recorder.read_bits(8); }

                let mut code = String::new();
                let mut valid = true;
                for _ in 0..4 {
                    if let Ok(ch) = huffman.decode_recorded(&mut h_recorder) {
                        code.push(ch);
                    } else {
                        valid = false;
                        break;
                    }
                }

                if valid && is_plausible_item_header(mode, loc, &code, flags, version, true) {
                    println!("  [FOUND] at bit {}: code='{}', flags={:#010x}, ver={}, mode={}, loc={}", 
                        bit_pos, code, flags, version, mode, loc);
                    break;
                }
            }
        }
    }

    Ok(())
}
