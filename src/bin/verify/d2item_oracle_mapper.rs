use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, is_plausible_item_header, peek_item_header_at};
use d2r_core::save::find_jm_markers;
use serde::Serialize;
use std::fs;
use std::io::{self, Cursor};

use d2r_core::report::Report;

#[derive(Serialize, Debug, Clone)]
struct ScanAnchor {
    bit_offset: u64,
    code: String,
    flags: u32,
    version: u8,
    mode: u8,
    location: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    best_width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<i32>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.contains(&"--help".to_string()) {
        println!("Usage: d2item_oracle_mapper <save_path> [options]");
        println!("Options:");
        println!("  --list-anchors       List all plausible JM item anchors");
        println!("  --auto-map           Mathematically infer bit-widths for all items");
        println!("  --json               Output result as JSON");
        return Ok(());
    }

    let path = &args[1];
    let bytes = fs::read(path)?;
    let huffman = HuffmanTree::new();

    let list_anchors = args.contains(&"--list-anchors".to_string());
    let auto_map = args.contains(&"--auto-map".to_string());
    let heatmap = args.contains(&"--heatmap".to_string());
    let show_json = args.contains(&"--json".to_string());

    let mut heatmap_width = 10;
    if let Some(pos) = args.iter().position(|x| x == "--width") {
        if let Some(w) = args.get(pos + 1) {
            heatmap_width = w.parse().unwrap_or(10);
        }
    }

    let mut scan_results = Vec::new();

    let jm_markers = find_jm_markers(&bytes);
    for &jm_pos in &jm_markers {
        let section_start = (jm_pos + 4) * 8;
        let mut bit_cursor = section_start as u64;
        let section_end = (bytes.len() * 8) as u64;

        while bit_cursor < section_end {
            if let Some((mode, loc, code, flags, ver, _compact, header_bits)) =
                peek_item_header_at(&bytes, bit_cursor, &huffman, true)
            {
                if is_plausible_item_header(mode, loc, &code, flags, ver, true) {
                    let mut record = ScanAnchor {
                        bit_offset: bit_cursor,
                        code: code.trim().to_string(),
                        flags,
                        version: ver,
                        mode,
                        location: loc,
                        best_width: None,
                        score: None,
                    };

                    if auto_map {
                        // Slice 2: Scoring Engine
                        // In Alpha v105, there is a ~19 bit overhead after the code for extended stats
                        // (Level, Quality, Flags) before the property list starts.
                        let stats_offset = if record.version == 5 || record.version == 1 {
                            19
                        } else {
                            0
                        };
                        let (width, score) =
                            infer_bit_width(&bytes, bit_cursor + header_bits + stats_offset, ver);
                        record.best_width = Some(width);
                        record.score = Some(score);
                    }

                    scan_results.push(record);
                    bit_cursor += header_bits;
                    // skip alignment
                    continue;
                }
            }
            bit_cursor += 1;
        }
    }

    if list_anchors || auto_map {
        if show_json {
            let report = Report::new(path, scan_results.clone());
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!("| Anchor Bit | Code | Flags      | Ver | Mode | Loc | Width | Score |");
            println!("|------------|------|------------|-----|------|-----|-------|-------|");
            for a in &scan_results {
                let width_str = a
                    .best_width
                    .map(|w| w.to_string())
                    .unwrap_or_else(|| "-".to_string());
                let score_str = a
                    .score
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "-".to_string());
                println!(
                    "| {:10} | {:4} | {:#010x} | {:3} | {:4} | {:3} | {:5} | {:5} |",
                    a.bit_offset,
                    a.code,
                    a.flags,
                    a.version,
                    a.mode,
                    a.location,
                    width_str,
                    score_str
                );
            }
        }
    }

    if heatmap && !show_json {
        for a in &scan_results {
            if a.code == "gp" {
                continue;
            } // Skip empty section
            println!(
                "\n[Heatmap] Code: {}, Start: {}, Width: {}",
                a.code, a.bit_offset, heatmap_width
            );
            let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
            let _stats_offset = if a.version == 5 || a.version == 1 {
                19
            } else {
                0
            };

            // Search for the actual start of stats bit (heuristic to find first 1 or 0x1FF)
            let start = a.bit_offset + 60; // Approximate header end
            if reader.skip(start as u32).is_err() {
                continue;
            }

            for row in 0..10 {
                print!("  Row {:2}: ", row);
                for _ in 0..heatmap_width {
                    let bit = reader.read_bit().unwrap_or(false);
                    print!("{}", if bit { "1" } else { "0" });
                }
                println!();
            }
        }
    }

    Ok(())
}

fn infer_bit_width(bytes: &[u8], stats_start_bit: u64, _version: u8) -> (u32, i32) {
    let mut best_width = 10;
    let mut best_score = -10000;

    for width in 8..=24 {
        let mut score = 0;
        let mut found_terminator = false;

        let mut reader = BitReader::endian(Cursor::new(bytes), LittleEndian);
        if reader.skip(stats_start_bit as u32).is_err() {
            continue;
        }

        for _p in 0..128 {
            let stat_id = match reader.read_var::<u16>(9) {
                Ok(id) => id,
                Err(_) => break,
            };

            if stat_id == 0x1FF {
                found_terminator = true;
                // Strong weight for terminator. Use the property count to anchor the probability.
                score += 1000;
                // We also expect items to have a reasonable number of properties, but shorter is more common.
                // However, we don't want to over-favor 0-length if it's junk.
                break;
            }

            // Statistical validity: Stats in Alpha are mostly < 512.
            if stat_id < 512 {
                score += 20;
            } else {
                // Harsh penalty for high-bit stat_ids (likely desync)
                score -= 1000;
                break;
            }

            let val_bits = (width as i32 - 9).max(0) as u32;
            if reader.skip(val_bits).is_err() {
                break;
            }
        }

        if !found_terminator {
            score -= 5000; // Severe penalty for missing terminator in expected window
        }

        // Special check for v105: 10-bit is the ground truth for non-compact items.
        // We break ties by favoring 10-bit if scores are very close.
        if width == 10 {
            score += 1;
        }

        if score > best_score {
            best_score = score;
            best_width = width;
        }
    }

    (best_width, best_score)
}
