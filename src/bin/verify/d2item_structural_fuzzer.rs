use std::fs;
use std::io::{self, Cursor};
use bitstream_io::{BitRead, BitReader, LittleEndian};

/// Bitstream Structural Fuzzer & Analyzer for D2R Alpha v105
/// Rank bit-width candidates based on terminator alignment.
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.contains(&"--help".to_string()) || args.contains(&"-h".to_string()) {
        println!("Usage: d2item_structural_fuzzer <save_path> [options]");
        println!("Options:");
        println!("  --start-bit <n>      Start bit offset (default: 0)");
        println!("  --width-range <n..m> Bit width range to sweep (default: 10..25)");
        println!("  --max-props <n>      Maximum properties to read per width (default: 64)");
        return Ok(());
    }

    let path = &args[1];
    let mut start_bit: u64 = 0;
    let mut brute_mode = false;
    let mut width_range_start = 10;
    let mut width_range_end = 25;
    let mut max_props = 64;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--start-bit" => {
                if i + 1 < args.len() {
                    start_bit = args[i+1].parse().expect("Invalid start-bit");
                    i += 2;
                } else {
                    panic!("Missing value for --start-bit");
                }
            }
            "--brute" => {
                brute_mode = true;
                i += 1;
            }
            "--width-range" => {
                if i + 1 < args.len() {
                    let range: Vec<&str> = args[i+1].split("..").collect();
                    if range.len() == 2 {
                        width_range_start = range[0].parse().expect("Invalid range start");
                        width_range_end = range[1].parse().expect("Invalid range end");
                    } else {
                        panic!("Invalid range format (expected n..m)");
                    }
                    i += 2;
                } else {
                    panic!("Missing value for --width-range");
                }
            }
            "--max-props" => {
                if i + 1 < args.len() {
                    max_props = args[i+1].parse().expect("Invalid max-props");
                    i += 2;
                } else {
                    panic!("Missing value for --max-props");
                }
            }
            _ => {
                println!("Warning: Unknown option: {}", args[i]);
                i += 1;
            }
        }
    }

    let bytes = fs::read(path)?;
    println!("[StructuralFuzzer] File: {}", path);
    println!("  Start Bit: {}", start_bit);
    println!("  Brute Mode: {}", brute_mode);
    println!("  Width Range: {}..{}", width_range_start, width_range_end);
    println!("  Max Props: {}", max_props);

    let mut best_width = 0;
    let mut best_score = -1;
    let mut best_start = start_bit;

    let scan_range = if brute_mode { 1000 } else { 1 };

    println!("\nScanning...");
    for current_start in start_bit..(start_bit + scan_range) {
        for width in width_range_start..=width_range_end {
            let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
            if let Err(_) = reader.skip(current_start as u32) {
                continue;
            }

            let mut found_terminator = false;
            let mut props_read = 0;
            let mut terminator_at = 0;

            for p in 0..max_props {
                let current_pos = current_start + (p as u64 * width as u64);
                let stat_id = match reader.read_var::<u16>(9) {
                    Ok(id) => id,
                    Err(_) => break,
                };

                if stat_id == 0x1FF && !found_terminator {
                    found_terminator = true;
                    props_read = p;
                    terminator_at = current_pos;
                }

                if width < 9 { break; }
                if let Err(_) = reader.read_var::<u64>((width - 9) as u32) { break; }
            }

            if found_terminator {
                if brute_mode && props_read > 5 {
                    println!("  Match Found: Start={}, Width={}, Props={}", current_start, width, props_read);
                }
                
                if props_read as i32 > best_score {
                    best_score = props_read as i32;
                    best_width = width;
                    best_start = current_start;
                }
            }
        }
    }

    start_bit = best_start; // Use best start for reporting

    if best_width > 0 {
        println!("\nBest Fit Width: {}", best_width);
        println!("------------------------------------------------------------------");
        println!("| Index | Bit Offset | Stat ID (Hex/Dec) | Raw Value (Hex/Dec)   |");
        println!("------------------------------------------------------------------");

        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        if let Err(_) = reader.skip(start_bit as u32) {
            println!("Error re-reading best fit width.");
        } else {
            for p in 0..max_props {
                let current_pos = start_bit + (p as u64 * best_width as u64);
                let stat_id = match reader.read_var::<u16>(9) {
                    Ok(id) => id,
                    Err(_) => break,
                };

                if stat_id == 0x1FF {
                    println!("| {:5} | {:10} | [ TERMINATOR ]    |                       |", p, current_pos);
                    // Decide if we want to break or continue to see what's after
                    // For now, let's continue for a few more to see spacers
                } else {
                    let value_bits = (best_width - 9) as u32;
                    let value = reader.read_var::<u64>(value_bits).unwrap_or(0);
                    
                    println!("| {:5} | {:10} | {:#05x} ({:4})    | {:#010x} ({:10}) |", 
                        p, current_pos, stat_id, stat_id, value, value);
                }
            }
        }
        println!("------------------------------------------------------------------");
    } else {
        println!("\nNo valid width found with a terminator.");
    }

    Ok(())
}
