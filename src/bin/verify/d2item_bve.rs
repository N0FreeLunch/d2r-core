use std::env;
use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::io::Cursor;
use serde::Serialize;
use anyhow::{Result, Context};

#[derive(Serialize, Clone)]
struct BveCandidate {
    width: u32,
    raw_value: u32,
    score: i32,
    reasons: Vec<String>,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_help();
        return Ok(());
    }

    let hex_input = &args[1];
    let bit_offset: u64 = args[2].parse().context("Invalid bit offset")?;
    let use_json = args.contains(&"--json".to_string());

    let hex_clean = hex_input.trim_start_matches("0x");
    let bytes = hex::decode(hex_clean).context("Invalid hex input")?;
    
    let mut candidates = Vec::new();

    for width in 1..=32 {
        if let Ok(candidate) = evaluate_width(&bytes, bit_offset, width) {
            candidates.push(candidate);
        }
    }

    // Sort by score descending, then by width ascending
    candidates.sort_by(|a, b| {
        b.score.cmp(&a.score).then(a.width.cmp(&b.width))
    });

    if use_json {
        println!("{}", serde_json::to_string_pretty(&candidates)?);
    } else {
        println!("BVE - Bit-Field Value Explorer");
        println!("Offset: {} bits", bit_offset);
        println!();
        println!("+-------+------------+-------+----------------------------------------------------+");
        println!("| Width | Value      | Score | Reasons                                            |");
        println!("+-------+------------+-------+----------------------------------------------------+");
        for cand in candidates.iter().take(10) {
            let reasons = cand.reasons.join(", ");
            println!("| {:<5} | {:<10} | {:<5} | {:<50} |", cand.width, cand.raw_value, cand.score, reasons);
        }
        println!("+-------+------------+-------+----------------------------------------------------+");
    }

    Ok(())
}

fn evaluate_width(bytes: &[u8], offset: u64, width: u32) -> Result<BveCandidate> {
    let mut reader = BitReader::endian(Cursor::new(bytes), LittleEndian);
    reader.skip(offset as u32).context("Offset out of bounds")?;
    
    // Read bits one by one to support dynamic width without const generic issues
    let mut val: u32 = 0;
    for i in 0..width {
        let bit = reader.read_bit().context("Bit read failed")?;
        if bit {
            val |= 1 << i;
        }
    }
    
    let mut score = 0;
    let mut reasons = Vec::new();

    // 1. Common D2 Widths
    let common_widths = vec![7, 9, 10, 12, 21, 32];
    if common_widths.contains(&width) {
        score += 10;
        reasons.push(format!("Common width ({})", width));
    }

    // 2. Value 0 is very common for optional/padding fields
    if val == 0 {
        score += 5;
        reasons.push("Value is 0".to_string());
    }

    // 3. Power of 2 (Possible flag or enum)
    if val > 0 && (val & (val - 1)) == 0 {
        score += 5;
        reasons.push("Power of 2".to_string());
    }

    // 4. Plausible ID ranges
    if width >= 9 && val < 1024 {
        score += 3;
        reasons.push("Plausible ID range".to_string());
    }

    // 5. Plausible Stat range
    if width <= 10 && val < 256 {
        score += 3;
        reasons.push("Plausible small value range".to_string());
    }

    // 6. Typical property values (usually not huge)
    if width > 1 && val < (1 << (width - 1)) {
        score += 2;
        reasons.push("Value doesn't fill width".to_string());
    }

    Ok(BveCandidate {
        width,
        raw_value: val,
        score,
        reasons,
    })
}

fn print_help() {
    println!("Usage: d2item_bve <hex_binary> <bit_offset> [--json]");
    println!();
    println!("Example: d2item_bve 0x4A4D... 16");
}
