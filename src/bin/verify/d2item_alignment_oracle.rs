use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::verify::args::{ArgParser, ArgSpec};
use serde::Serialize;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Cursor;

#[derive(Serialize)]
struct AlignmentReport {
    total_markers: usize,
    intervals: Vec<u64>,
    distribution: HashMap<u64, usize>,
    inferred_alignment: Option<u64>,
}

fn main() {
    let mut parser = ArgParser::new("d2item_alignment_oracle");
    parser.add_spec(ArgSpec::positional("save_file", "Path to save file"));
    parser.add_spec(
        ArgSpec::positional("offset", "Bit offset from start to begin search")
            .optional()
            .with_default("0"),
    );
    parser.add_spec(ArgSpec::flag("json", None, Some("json"), "Output results in JSON format"));

    use d2r_core::verify::args::ArgError;
    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            std::process::exit(0);
        }
        Err(ArgError::Error(e)) => {
            eprintln!("error: {}", e);
            eprintln!("\n{}", parser.usage());
            std::process::exit(1);
        }
    };

    let path = parsed.get("save_file").unwrap();
    let offset = parsed
        .get("offset")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let use_json = parsed.is_set("json");

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            std::process::exit(1);
        }
    };

    let jm_pos = match (0..bytes.len() - 2).find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M') {
        Some(pos) => pos,
        None => {
            eprintln!("No initial JM marker found (base search anchor).");
            std::process::exit(1);
        }
    };

    if !use_json {
        println!("Starting exhaustive bit-level JM search from byte {} (bit {})...", jm_pos, jm_pos * 8);
        println!("Marker Pattern: 0x4D4A ('JM' in LittleEndian)");
        println!("{:-<60}", "");
        println!("{:>12} | {:>10} | {:>10}", "Bit Offset", "Abs Byte", "Interval");
        println!("{:-<60}", "");
    }

    let start_bit = (jm_pos * 8) as u64 + offset;
    let total_bits = (bytes.len() * 8) as u64;
    
    let mut last_marker_pos = start_bit;
    let mut found_count = 0;
    let mut intervals = Vec::new();

    for bit_idx in start_bit..total_bits - 16 {
        let byte_start = (bit_idx / 8) as usize;
        let bit_shift = (bit_idx % 8) as u32;
        
        let found = if byte_start + 4 > bytes.len() {
            // Near EOF, read carefully
            let mut buf = [0u8; 4];
            let remaining = bytes.len() - byte_start;
            buf[..remaining].copy_from_slice(&bytes[byte_start..]);
            let mut reader = BitReader::endian(Cursor::new(&buf), LittleEndian);
            let _ = reader.skip(bit_shift);
            reader.read::<16, u16>().unwrap_or(0) == 0x4D4A
        } else {
            let mut reader = BitReader::endian(Cursor::new(&bytes[byte_start..byte_start + 4]), LittleEndian);
            let _ = reader.skip(bit_shift);
            reader.read::<16, u16>().unwrap_or(0) == 0x4D4A
        };

        if found {
            let interval = bit_idx - last_marker_pos;
            if !use_json {
                println!("{:12} | {:10.2} | {:10}", bit_idx, bit_idx as f64 / 8.0, interval);
            }
            if found_count > 0 {
                intervals.push(interval);
            }
            last_marker_pos = bit_idx;
            found_count += 1;
        }
    }

    let mut distribution = HashMap::new();
    for &val in &intervals {
        *distribution.entry(val).or_insert(0) += 1;
    }

    let inferred_alignment = distribution.iter()
        .max_by_key(|&(_, count)| count)
        .map(|(&val, _)| val);

    if use_json {
        let report = AlignmentReport {
            total_markers: found_count,
            intervals,
            distribution,
            inferred_alignment,
        };
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("{:-<60}", "");
        println!("Search complete. Found {} markers.", found_count);
        if let Some(align) = inferred_alignment {
            println!("Inferred Alignment Rule: {} bits (most frequent interval)", align);
        }
    }
}
