use d2r_core::item::{HuffmanTree, Item, BitRecorder, RecordedBit};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use serde::Serialize;
use std::env;
use std::fs;
use std::io::{self, Cursor};
use std::path::PathBuf;

#[derive(Serialize, Debug, Clone)]
struct BitInfo {
    bit: bool,
    context: String,
}

#[derive(Serialize, Debug, Clone)]
struct BitDiff {
    bit_offset: u64,
    actual: bool,
    expected: bool,
    context: String,
}

#[derive(Serialize, Debug)]
struct SymmetryReport {
    file_path: String,
    item_index: usize,
    item_code: String,
    similarity: f64,
    status: String,
    diffs: Vec<BitDiff>,
    actual_len: usize,
    expected_len: usize,
}

impl std::fmt::Display for SymmetryReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "=== Symmetry Report: {} (Item #{}: {}) ===", 
            self.file_path, self.item_index, self.item_code.trim())?;
        writeln!(f, "Status     : {}", self.status)?;
        writeln!(f, "Similarity : {:.2}%", self.similarity)?;
        writeln!(f, "Actual bits: {}", self.actual_len)?;
        writeln!(f, "Expect bits: {}", self.expected_len)?;
        
        if !self.diffs.is_empty() {
            writeln!(f, "\nDiscrepancies (First 10):")?;
            for (idx, diff) in self.diffs.iter().enumerate().take(10) {
                writeln!(f, "  #{:02} [Bit {:4}] Expected: {}, Actual: {} | Context: {}", 
                    idx, diff.bit_offset, if diff.expected {1} else {0}, if diff.actual {1} else {0}, diff.context)?;
            }
        } else {
            writeln!(f, "\nPerfect Symmetry Achieved! (100.00%)")?;
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    
    let mut save_path_str = None;
    let mut item_index = None;
    let mut use_json = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => use_json = true,
            _ => {
                if save_path_str.is_none() {
                    save_path_str = Some(&args[i]);
                } else if item_index.is_none() {
                    item_index = Some(args[i].parse::<usize>().unwrap_or(0));
                }
            }
        }
        i += 1;
    }

    if save_path_str.is_none() || item_index.is_none() {
        println!("Usage: cargo run --bin SymmetryBitDiff -- <save_file> <item_index> [--json]");
        return Ok(());
    }

    let save_path = save_path_str.unwrap();
    let item_idx = item_index.unwrap();
    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();

    // 1. Load items from save
    let items = load_items_with_recorder(&bytes, &huffman);
    if item_idx >= items.len() {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("Item index {} not found. Total items: {}", item_idx, items.len())));
    }

    let (item, actual_bits) = &items[item_idx];
    
    // 2. Generate expected bits (Re-serialize)
    let is_v105 = item.version == 105;
    let mut cloned_item = item.clone();
    cloned_item.bits.clear(); // Clear recorded bits to force re-serialization
    let expected_bytes = cloned_item.to_bytes(&huffman, is_v105)?;
    
    let mut reader = BitReader::endian(Cursor::new(&expected_bytes), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    let _ = Item::from_reader_with_context(&mut recorder, &huffman, None, false);
    
    let expected_bits: Vec<BitInfo> = recorder.recorded_bits.iter().map(|rb| {
        // Here we'd ideally have context, but BitRecorder currently doesn't store context per bit
        // Let's assume context mapping for now
        BitInfo { bit: rb.bit, context: "Oracle".to_string() }
    }).collect();

    // 3. Compare
    let report = generate_report(save_path, item_idx, item.code.clone(), actual_bits, &expected_bits);

    if use_json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("{}", report);
    }

    // Always save trace to tmp/
    let _ = fs::create_dir_all("tmp");
    let trace_path = format!("tmp/symmetry_report_item_{}.json", item_idx);
    let _ = fs::write(trace_path, serde_json::to_string_pretty(&report).unwrap());

    Ok(())
}

fn load_items_with_recorder(bytes: &[u8], huffman: &HuffmanTree) -> Vec<(Item, Vec<BitInfo>)> {
    let mut collection = Vec::new();
    let mut jm_pos = 0;
    while let Some(rel_jm) = bytes[jm_pos..].windows(2).position(|w| w == b"JM") {
        let abs_jm = jm_pos + rel_jm;
        if abs_jm + 4 <= bytes.len() {
             let mut bit_pos = (abs_jm as u64 + 4) * 8;
             let bit_limit = bytes.len() as u64 * 8;
             
             while bit_pos < bit_limit - 16 {
                let b_start = (bit_pos / 8) as usize;
                let b_off = (bit_pos % 8) as u32;
                let mut cursor = Cursor::new(&bytes[b_start..]);
                let mut reader = BitReader::endian(&mut cursor, LittleEndian);
                if b_off > 0 { let _ = reader.skip(b_off).ok(); }
                
                let mut recorder = BitRecorder::new(&mut reader);
                match Item::from_reader_with_context(&mut recorder, huffman, None, false) {
                    Ok(item) => {
                        let consumed = recorder.total_read;
                        let bits = recorder.recorded_bits.iter().map(|rb| {
                            BitInfo { bit: rb.bit, context: "Actual".to_string() }
                        }).collect();
                        collection.push((item, bits));
                        bit_pos += consumed;
                    }
                    Err(_) => {
                        bit_pos += 1;
                    }
                }
             }
        }
        jm_pos = abs_jm + 2;
        if !collection.is_empty() { break; }
    }
    collection
}

fn generate_report(
    path: &str, 
    index: usize, 
    code: String, 
    actual: &[BitInfo], 
    expected: &[BitInfo]
) -> SymmetryReport {
    let mut diffs = Vec::new();
    let max_len = actual.len().max(expected.len());
    let mut matches = 0;

    for i in 0..max_len {
        let a = actual.get(i);
        let e = expected.get(i);
        
        match (a, e) {
            (Some(ab), Some(eb)) => {
                if ab.bit == eb.bit {
                    matches += 1;
                } else {
                    diffs.push(BitDiff {
                        bit_offset: i as u64,
                        actual: ab.bit,
                        expected: eb.bit,
                        context: ab.context.clone(),
                    });
                }
            }
            (Some(ab), None) => {
                diffs.push(BitDiff {
                    bit_offset: i as u64,
                    actual: ab.bit,
                    expected: false,
                    context: format!("Extra bit in Actual: {}", ab.context),
                });
            }
            (None, Some(eb)) => {
                diffs.push(BitDiff {
                    bit_offset: i as u64,
                    actual: false,
                    expected: eb.bit,
                    context: format!("Missing bit in Actual: {}", eb.context),
                });
            }
            _ => {}
        }
    }

    let similarity = if max_len > 0 {
        (matches as f64 / max_len as f64) * 100.0
    } else {
        0.0
    };

    let status = if similarity >= 100.0 { "PERFECT" } else if similarity >= 90.0 { "HIGH_SIMILARITY" } else { "DESYNC" };

    SymmetryReport {
        file_path: path.to_string(),
        item_index: index,
        item_code: code,
        similarity,
        status: status.to_string(),
        diffs,
        actual_len: actual.len(),
        expected_len: expected.len(),
    }
}
