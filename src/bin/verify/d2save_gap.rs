use std::env;
use std::fs;
use std::io::Cursor;
use anyhow::{Result, Context};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use serde::Serialize;

use d2r_core::verify::args::{ArgParser, ArgSpec};
use d2r_core::item::HuffmanTree;
use d2r_core::data::bit_cursor::BitCursor;

#[derive(Debug, Serialize, Clone)]
struct Marker {
    abs_bit: u64,
    drift: u64,
}

#[derive(Debug, Serialize, Clone)]
struct Candidate {
    length: u32,
    score: f64,
    readability_score: f64,
    code: String,
    markers: Vec<Marker>,
}

#[derive(Debug, Serialize, Clone)]
struct StatResult {
    id_w: u32,
    val_w: u32,
    depth: usize,
    terminator_pos: u64,
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
            let _ = reader.skip(bit_offset).unwrap_or(());
        }

        let b1: u8 = reader.read::<8, u8>().unwrap_or(0);
        let b2: u8 = reader.read::<8, u8>().unwrap_or(0);

        if b1 == 0x4A && b2 == 0x4D {
            markers.push(bit_pos);
        }
    }
    markers
}

fn read_bits_runtime<R: BitRead>(reader: &mut R, count: u32) -> std::io::Result<u32> {
    let mut val = 0u32;
    for i in 0..count {
        if reader.read_bit()? {
            val |= 1 << i;
        }
    }
    Ok(val)
}

fn main() -> Result<()> {
    let mut parser = ArgParser::new("d2save_gap");
    parser.add_spec(ArgSpec::positional("path", "Path to the d2s file"));
    parser.add_spec(ArgSpec::option("min", None, Some("min"), "Minimum header bit length").with_default("56"));
    parser.add_spec(ArgSpec::option("max", None, Some("max"), "Maximum header bit length").with_default("72"));
    parser.add_spec(ArgSpec::option("item-offset", None, Some("offset"), "Byte offset to item section"));
    parser.add_spec(ArgSpec::flag("probe-stats", None, Some("probe-stats"), "Enable stat bit-width probing"));
    parser.add_spec(ArgSpec::option("id-min", None, Some("id-min"), "Min ID bits").with_default("7"));
    parser.add_spec(ArgSpec::option("id-max", None, Some("id-max"), "Max ID bits").with_default("11"));
    parser.add_spec(ArgSpec::option("val-min", None, Some("val-min"), "Min Value bits").with_default("6"));
    parser.add_spec(ArgSpec::option("val-max", None, Some("val-max"), "Max Value bits").with_default("14"));
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
    let huffman = HuffmanTree::new();

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
        
        // 1. Readability Scoring
        let mut code = String::new();
        let mut readability_score = 0.0;
        
        let mut r_reader = BitReader::endian(Cursor::new(&bytes[(first_item_start / 8) as usize..]), LittleEndian);
        let skip_bits = (first_item_start % 8) as u32;
        if skip_bits > 0 {
            let _ = r_reader.skip(skip_bits).unwrap_or(());
        }
        let mut r_cursor = BitCursor::new(r_reader);
        
        for _ in 0..4 {
            match huffman.decode_recorded(&mut r_cursor) {
                Ok(ch) => {
                    code.push(ch);
                    if ch.is_alphanumeric() || ch == ' ' {
                        readability_score += 1.0;
                    } else {
                        readability_score -= 0.5;
                    }
                }
                Err(_) => {
                    readability_score -= 0.5;
                }
            }
        }
        
        // Code plausibility boost
        if code.len() == 4 && code.chars().all(|c| c.is_alphanumeric() || c == ' ') {
            if code.trim().len() >= 3 {
                readability_score += 500.0;
            }
        }

        // 2. Alignment Scoring (Drift)
        // Scan for subsequent JMs (limit to a reasonable range, e.g., 20000 bits)
        let found_bits = scan_jm_markers(&bytes, first_item_start, 20000);

        let mut markers = Vec::new();
        let mut alignment_score = 0.0;

        for abs_bit in found_bits {
            let drift = abs_bit % 8;
            alignment_score += 1.0 / (1.0 + drift as f64);
            markers.push(Marker { abs_bit, drift });
        }

        candidates.push(Candidate {
            length,
            score: alignment_score,
            readability_score,
            code,
            markers,
        });
    }

    candidates.sort_by(|a, b| {
        b.readability_score.partial_cmp(&a.readability_score).unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal))
            .then_with(|| a.length.cmp(&b.length))
    });

    if is_json {
        println!("{}", serde_json::to_string_pretty(&candidates)?);
    } else {
        println!("--- D2 Save GAP (Geometric Auto-Pilot) ---");
        println!("File: {}", path);
        println!("Item Section Heuristic Start: Byte {} (Bit {})", item_start_byte, item_start_bit);
        println!("\n{:<10} {:<10} {:<10} {:<10} {:<15} {:<15}", "Header", "R-Score", "Score", "Markers", "Code", "Drifts");
        for c in candidates.iter().take(20) {
            let drifts: Vec<String> = c.markers.iter().take(5).map(|m| m.drift.to_string()).collect();
            let drift_str = drifts.join(",");
            println!("{:<10} {:<10.1} {:<10.4} {:<10} {:<15} {:<15}", c.length, c.readability_score, c.score, c.markers.len(), c.code, drift_str);
        }
    }

    if parsed.is_set("probe-stats") && !candidates.is_empty() {
        let id_min: u32 = parsed.get("id-min").unwrap().parse().context("Invalid id-min")?;
        let id_max: u32 = parsed.get("id-max").unwrap().parse().context("Invalid id-max")?;
        let val_min: u32 = parsed.get("val-min").unwrap().parse().context("Invalid val-min")?;
        let val_max: u32 = parsed.get("val-max").unwrap().parse().context("Invalid val-max")?;

        let best_header = candidates[0].length;
        let stat_start_bit = item_start_bit + (best_header as u64) + 32;

        let mut stat_results = Vec::new();

        for id_w in id_min..=id_max {
            for val_w in val_min..=val_max {
                let mut reader = BitReader::endian(Cursor::new(&bytes[(stat_start_bit / 8) as usize..]), LittleEndian);
                let skip = (stat_start_bit % 8) as u32;
                if skip > 0 { let _ = reader.skip(skip).unwrap_or(()); }

                let mut depth = 0;
                let terminator = (1 << id_w) - 1;
                let mut current_pos = stat_start_bit;

                loop {
                    let id: u32 = match read_bits_runtime(&mut reader, id_w) {
                        Ok(v) => v,
                        Err(_) => break,
                    };
                    current_pos += id_w as u64;

                    if id == terminator {
                        stat_results.push(StatResult { id_w, val_w, depth, terminator_pos: current_pos });
                        break;
                    }

                    match reader.skip(val_w) {
                        Ok(_) => {},
                        Err(_) => break,
                    }
                    current_pos += val_w as u64;
                    depth += 1;

                    if depth > 200 || current_pos > stat_start_bit + 5000 {
                        break;
                    }
                }
            }
        }

        stat_results.sort_by(|a, b| b.depth.cmp(&a.depth).then_with(|| a.id_w.cmp(&b.id_w)));

        if is_json {
            println!("{}", serde_json::to_string_pretty(&stat_results)?);
        } else {
            println!("\n--- Stat Probing (Header: {}) ---", best_header);
            println!("{:<10} {:<10} {:<10} {:<15}", "ID Bits", "Val Bits", "Depth", "Terminator Bit");
            for r in stat_results.iter().take(20) {
                println!("{:<10} {:<10} {:<10} {:<15}", r.id_w, r.val_w, r.depth, r.terminator_pos);
            }
        }
    }

    Ok(())
}

