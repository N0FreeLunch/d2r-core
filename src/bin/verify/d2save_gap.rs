use std::env;
use std::fs;
use std::io::Cursor;
use anyhow::{Result, Context};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use serde::Serialize;

use d2r_core::verify::args::{ArgParser, ArgSpec};

#[derive(Debug, Serialize, Clone)]
struct Marker {
    abs_bit: u64,
    drift: u64,
}

#[derive(Debug, Serialize, Clone)]
struct Candidate {
    length: u32,
    score: f64,
    markers: Vec<Marker>,
}

fn scan_jm_markers(bytes: &[u8], start_bit: u64, limit_bits: u64) -> Vec<u64> {
    let mut markers = Vec::new();
    let total_bits = (bytes.len() as u64) * 8;
    let end_bit = std::cmp::min(start_bit + limit_bits, total_bits.saturating_sub(16));

    for bit_pos in start_bit..end_bit {
        let byte_offset = (bit_pos / 8) as usize;
        let bit_offset = (bit_pos % 8) as u32;

        if byte_offset + 2 >= bytes.len() {
            break;
        }

        let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset..]), LittleEndian);
        if bit_offset > 0 {
            let _ = reader.skip(bit_offset);
        }

        let b1: u8 = reader.read::<8, u8>().unwrap_or(0);
        let b2: u8 = reader.read::<8, u8>().unwrap_or(0);

        if b1 == 0x4A && b2 == 0x4D {
            markers.push(bit_pos);
        }
    }
    markers
}

fn main() -> Result<()> {
    let mut parser = ArgParser::new("d2save_gap");
    parser.add_spec(ArgSpec::positional("path", "Path to the d2s file"));
    parser.add_spec(ArgSpec::option("min", None, Some("min"), "Minimum header bit length").with_default("56"));
    parser.add_spec(ArgSpec::option("max", None, Some("max"), "Maximum header bit length").with_default("72"));
    parser.add_spec(ArgSpec::option("item-offset", None, Some("offset"), "Byte offset to item section"));
    parser.add_spec(ArgSpec::flag("json", None, Some("json"), "Output in JSON format"));

    let args: Vec<_> = env::args_os().skip(1).collect();

    use d2r_core::verify::args::ArgError;
    let parsed = match parser.parse(args) {
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

    let path = parsed.get("path").cloned().unwrap();
    let min_len: u32 = parsed.get("min").unwrap().parse().context("Invalid min length")?;
    let max_len: u32 = parsed.get("max").unwrap().parse().context("Invalid max length")?;
    let item_offset_arg = parsed.get("item-offset").map(|s| s.parse::<usize>()).transpose().context("Invalid item-offset")?;
    let is_json = parsed.is_set("json");

    let bytes = fs::read(&path).with_context(|| format!("Failed to read file: {}", path))?;

    // Find item section start
    let item_start_byte = if let Some(offset) = item_offset_arg {
        offset
    } else {
        // Heuristic: search for first JM
        let mut found = None;
        for i in 0..(bytes.len().saturating_sub(2)) {
            if bytes[i] == 0x4A && bytes[i+1] == 0x4D {
                found = Some(i);
                break;
            }
        }
        found.context("Could not find item section (no JM marker found)")?
    };

    let item_start_bit = (item_start_byte as u64) * 8;
    let mut candidates = Vec::new();

    for length in min_len..=max_len {
        let first_item_start = item_start_bit + (length as u64);
        // Scan for subsequent JMs (limit to a reasonable range, e.g., 20000 bits)
        let found_bits = scan_jm_markers(&bytes, first_item_start, 20000);

        let mut markers = Vec::new();
        let mut score = 0.0;

        for abs_bit in found_bits {
            let drift = abs_bit % 8;
            score += 1.0 / (1.0 + drift as f64);
            markers.push(Marker { abs_bit, drift });
        }

        candidates.push(Candidate {
            length,
            score,
            markers,
        });
    }

    candidates.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.length.cmp(&b.length))
    });

    if is_json {
        println!("{}", serde_json::to_string_pretty(&candidates)?);
    } else {
        println!("--- D2 Save GAP (Geometric Auto-Pilot) ---");
        println!("File: {}", path);
        println!("Item Section Heuristic Start: Byte {} (Bit {})", item_start_byte, item_start_bit);
        println!("\n{:<10} {:<10} {:<10} {:<15}", "Header", "Score", "Markers", "Drifts");
        for c in candidates.iter().take(20) {
            let drifts: Vec<String> = c.markers.iter().take(5).map(|m| m.drift.to_string()).collect();
            let drift_str = drifts.join(",");
            println!("{:<10} {:<10.4} {:<10} {:<15}", c.length, c.score, c.markers.len(), drift_str);
        }
    }

    Ok(())
}
