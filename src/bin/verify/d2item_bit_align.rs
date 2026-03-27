use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::algo::alignment::BitAligner;
use d2r_core::item::{BitRecorder, HuffmanTree, Item};
use d2r_core::report::Report;
use serde::Serialize;
use std::env;
use std::fs;
use std::io::{self, Cursor};
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
struct ScanEntry {
    code: String,
    bit_offset: u64,
    len: u64,
    is_error: bool,
}

#[derive(Serialize, Debug)]
struct BitAlignReport {
    item_index: usize,
    code: String,
    actual_bits: usize,
    expected_bits: usize,
    similarity_pct: f64,
    gap_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    visualization: Option<String>,
}

fn main() -> io::Result<()> {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!(
            "CLI Usage: cargo run --bin d2item_bit_align -- <save_file_path> <item_index> [jm_offset_hex] [--json]"
        );
        return Ok(());
    }

    let is_json = args.contains(&"--json".to_string());
    let save_path = &args[1];
    let item_index: usize = args[2]
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid item index"))?;

    let mut jm_offset_override = None;
    if args.len() >= 4 {
        // If the 4th arg is not --json, treat it as offset.
        // If it is --json, then we only have 3 positional args.
        if args[3] != "--json" {
            jm_offset_override = Some(
                usize::from_str_radix(args[3].trim_start_matches("0x"), 16).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid hex offset")
                })?,
            );
        }
    }

    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();

    let mut scan_results = Vec::new();
    let items = if let Some(offset) = jm_offset_override {
        if !is_json {
            println!(
                "[d2item_bit_align] Scanning at forced JM offset 0x{:04X}...",
                offset
            );
        }
        scan_at_offset(
            &bytes[(offset + 4)..],
            &huffman,
            &mut Vec::new(),
            &mut scan_results,
            is_json,
        )
    } else {
        if !is_json {
            println!("[d2item_bit_align] Scanning items...");
        }
        load_items_scanning(&bytes, &huffman, &mut scan_results, is_json)
    };

    if item_index >= items.len() {
        if is_json {
            let report = serde_json::json!({
                "error": format!("Item index {} out of bounds. Found {} items.", item_index, items.len()),
                "found_count": items.len(),
                "scan_results": scan_results
            });
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!(
                "[d2item_bit_align] Error: Item index {} out of bounds. Found {} items.",
                item_index,
                items.len()
            );
            if !items.is_empty() {
                println!("Available items:");
                for (i, it) in items.iter().enumerate() {
                    println!(
                        "  #{}: {} (ver={}, loc={}, mode={}, bits={})",
                        i,
                        it.code.trim(),
                        it.version,
                        it.location,
                        it.mode,
                        it.bits.len()
                    );
                }
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Item index out of range",
        ));
    }

    let item = &items[item_index];
    let actual: Vec<bool> = item.bits.iter().map(|rb| rb.bit).collect();

    if actual.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No bits recorded for the item.",
        ));
    }

    // Strategy A: Re-serialize and re-parse to get "Expected Bits"
    let mut expected_item = item.clone();
    expected_item.bits.clear(); // Force re-encoding
    let expected_encoded_bytes = expected_item.to_bytes(&huffman, expected_item.version == 105)?;

    let mut reader = BitReader::endian(Cursor::new(&expected_encoded_bytes), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    let _ = Item::from_reader_with_context(&mut recorder, &huffman, None, false).ok();
    let expected: Vec<bool> = recorder.recorded_bits.iter().map(|rb| rb.bit).collect();

    let aligner = BitAligner::new(2, -1, -3, -1);
    let result = aligner.align(&actual, &expected);

    let detail = BitAlignReport {
        item_index,
        code: item.code.trim().to_string(),
        actual_bits: actual.len(),
        expected_bits: expected.len(),
        similarity_pct: result.similarity_pct(),
        gap_count: result.gap_indices.len(),
        visualization: if result.similarity_pct() < 100.0 {
            Some(result.pretty_print())
        } else {
            None
        },
    };

    if is_json {
        let report = Report::new(save_path, scan_results);
        let json_report = serde_json::json!({
            "file": report.metadata.file,
            "item_index": detail.item_index,
            "code": detail.code,
            "scan_results": report.scan_results,
            "actual_bits": detail.actual_bits,
            "expected_bits": detail.expected_bits,
            "similarity_pct": detail.similarity_pct,
            "gap_count": detail.gap_count,
            "visualization": detail.visualization
        });
        println!("{}", serde_json::to_string_pretty(&json_report).unwrap());
    } else {
        let metadata = d2r_core::report::ReportMetadata::from_path(save_path);
        println!(
            "[d2item_bit_align] Save: {} | Item #{} ({})",
            metadata.file, detail.item_index, detail.code
        );
        println!("  Actual  bits : {}", detail.actual_bits);
        println!("  Expected bits: {}", detail.expected_bits);
        println!("  Similarity   : {:.2}%", detail.similarity_pct);
        println!("  Gap count    : {}", detail.gap_count);

        if let Some(viz) = &detail.visualization {
            println!("\nAlignment Visualization:");
            println!("{}", viz);
        } else {
            println!("  Perfect match (100.00%)!");
        }
    }

    Ok(())
}

fn load_items_scanning(
    bytes: &[u8],
    huffman: &HuffmanTree,
    scan_results: &mut Vec<ScanEntry>,
    is_json: bool,
) -> Vec<Item> {
    let mut all_items = Vec::new();

    // Find JM item section
    let mut jm_pos = 0;
    while let Some(rel_jm) = bytes[jm_pos..].windows(2).position(|w| w == b"JM") {
        let abs_jm = jm_pos + rel_jm;
        if abs_jm + 4 <= bytes.len() {
            scan_at_offset(
                &bytes[(abs_jm + 4)..],
                huffman,
                &mut all_items,
                scan_results,
                is_json,
            );
        }
        jm_pos = abs_jm + 2;
        if !all_items.is_empty() {
            break;
        }
    }
    all_items
}

fn scan_at_offset(
    bytes: &[u8],
    huffman: &HuffmanTree,
    collection: &mut Vec<Item>,
    scan_results: &mut Vec<ScanEntry>,
    is_json: bool,
) -> Vec<Item> {
    let mut bit_pos = 0u64;
    let bit_limit = bytes.len() as u64 * 8;

    while bit_pos < bit_limit - 16 {
        // Lowered limit to catch small items/errors
        let b_start = (bit_pos / 8) as usize;
        let b_off = (bit_pos % 8) as u32;
        let mut cursor = Cursor::new(&bytes[b_start..]);
        let mut reader = BitReader::endian(&mut cursor, LittleEndian);
        if b_off > 0 {
            let _ = reader.skip(b_off).ok();
        }

        let mut recorder = BitRecorder::new(&mut reader);
        match Item::from_reader_with_context(&mut recorder, huffman, None, false) {
            Ok(item) => {
                let consumed = reader.position_in_bits().unwrap_or(0);
                if consumed >= 30 {
                    let version = item.version;
                    if !is_json {
                        println!(
                            "  [Scan OK]  found {} at bit offset {}, len={}",
                            item.code.trim(),
                            bit_pos,
                            consumed
                        );
                    }
                    scan_results.push(ScanEntry {
                        code: item.code.trim().to_string(),
                        bit_offset: bit_pos,
                        len: consumed,
                        is_error: false,
                    });
                    if version == 5 {
                        bit_pos = (bit_pos + consumed + 7) & !7;
                    } else {
                        bit_pos += consumed;
                    }
                    collection.push(item);
                } else {
                    bit_pos += 1;
                }
            }
            Err(_) => {
                let consumed = recorder.recorded_bits.len();
                if consumed >= 30 {
                    let mut item = Item::empty_for_tests();
                    item.code = "ERR ".to_string();
                    item.bits = recorder.recorded_bits.clone();
                    item.version = 5; // Default for alpha analysis
                    if !is_json {
                        println!(
                            "  [Scan ERR] found {} at bit offset {}, len={}",
                            item.code.trim(),
                            bit_pos,
                            consumed
                        );
                    }
                    scan_results.push(ScanEntry {
                        code: item.code.trim().to_string(),
                        bit_offset: bit_pos,
                        len: consumed as u64,
                        is_error: true,
                    });
                    collection.push(item);

                    // Also align to byte on error if version 5
                    bit_pos = (bit_pos + consumed as u64 + 7) & !7;
                } else {
                    bit_pos += 1;
                }
            }
        }
    }
    collection.clone()
}
