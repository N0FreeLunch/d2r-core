use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use d2r_core::item::{BitRecorder, HuffmanTree, Item, RecordedBit};
use serde::Serialize;
use std::env;
use std::fs;
use std::io::{self, Cursor};
use std::path::PathBuf;

#[derive(Serialize, Debug, Clone)]
struct NudgeTrial {
    nudge_offset: u64,
    shift_amount: i32,
    new_similarity: f64,
    new_status: String,
    code: String,
}

#[derive(Serialize, Debug)]
struct NudgeReport {
    file_path: String,
    target_item_index: usize,
    best_trial: Option<NudgeTrial>,
    trials: Vec<NudgeTrial>,
}

fn main() -> io::Result<()> {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();

    let mut save_path_str = None;
    let mut item_index = None;
    let mut target_offset = None;
    let mut nudge_range = 8;
    let mut use_json = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => use_json = true,
            "--offset" => {
                i += 1;
                target_offset = Some(args[i].parse::<u64>().unwrap_or(0));
            }
            "--range" => {
                i += 1;
                nudge_range = args[i].parse::<i32>().unwrap_or(8);
            }
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
        println!(
            "Usage: cargo run --bin BitNudgeExplorer -- <save_file> <item_index> [--offset <bit>] [--range <bits>] [--json]"
        );
        return Ok(());
    }

    let save_path = save_path_str.unwrap();
    let item_idx = item_index.unwrap();
    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();

    // 1. Get baseline from Slice 2 logic (re-implemented here for standalone use)
    let items = load_items_with_recorder(&bytes, &huffman);
    if item_idx >= items.len() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Item not found"));
    }
    let (item, actual_bits) = &items[item_idx];
    let offset = target_offset.unwrap_or(0); // If not provided, we might scan the whole item window later

    // 2. Perform Nudging trials
    let mut report = NudgeReport {
        file_path: save_path.clone(),
        target_item_index: item_idx,
        best_trial: None,
        trials: Vec::new(),
    };

    println!(
        "[BitNudgeExplorer] Exploring nudges for Item #{} ({}) near bit offset {}...",
        item_idx,
        item.code.trim(),
        offset
    );

    for shift in -nudge_range..=nudge_range {
        if shift == 0 {
            continue;
        }

        let nudged_bits = apply_nudge(actual_bits, offset, shift);
        let (sim, code) = evaluate_symmetry(&nudged_bits, &huffman, item.version == 105);

        let trial = NudgeTrial {
            nudge_offset: offset,
            shift_amount: shift,
            new_similarity: sim,
            new_status: if sim >= 100.0 { "SOLVED" } else { "TRIED" }.to_string(),
            code,
        };

        if report
            .best_trial
            .as_ref()
            .map_or(true, |b| trial.new_similarity > b.new_similarity)
        {
            report.best_trial = Some(trial.clone());
        }
        report.trials.push(trial);
    }

    if use_json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if let Some(best) = &report.best_trial {
            println!(
                "Best Nudge Result: Shift {} bits at Offset {} -> Similarity {:.2}% ({})",
                best.shift_amount, best.nudge_offset, best.new_similarity, best.new_status
            );
        } else {
            println!("No improvements found.");
        }
    }

    // Always save trace
    let _ = fs::create_dir_all("tmp");
    let trace_path = format!("tmp/bit_nudge_report_item_{}.json", item_idx);
    let _ = fs::write(trace_path, serde_json::to_string_pretty(&report).unwrap());

    Ok(())
}

fn apply_nudge(original: &[bool], offset: u64, shift: i32) -> Vec<bool> {
    let mut result = Vec::from(original);
    if shift > 0 {
        // Insert zero bits
        for _ in 0..shift {
            if (offset as usize) < result.len() {
                result.insert(offset as usize, false);
            } else {
                result.push(false);
            }
        }
    } else if shift < 0 {
        // Delete bits
        for _ in 0..shift.abs() {
            if (offset as usize) < result.len() {
                result.remove(offset as usize);
            }
        }
    }
    result
}

fn evaluate_symmetry(bits: &[bool], huffman: &HuffmanTree, is_v105: bool) -> (f64, String) {
    // 1. Convert bits to bytes for the reader
    let mut writer = BitWriter::endian(Vec::new(), LittleEndian);
    for &b in bits {
        let _ = writer.write_bit(b);
    }
    let nudged_bytes = writer.into_writer();

    // 2. Parse from nudged bytes
    let mut reader = BitReader::endian(Cursor::new(&nudged_bytes), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    match Item::from_reader_with_context(&mut recorder, huffman, None, false) {
        Ok(item) => {
            let actual_bits: Vec<bool> = recorder.recorded_bits.iter().map(|rb| rb.bit).collect();

            // 3. Re-serialize parsed item to get expected bits
            let mut cloned = item.clone();
            cloned.bits.clear();
            let Ok(expected_bytes) = cloned.to_bytes(huffman, is_v105) else {
                return (0.0, "ERR".to_string());
            };

            let mut e_reader = BitReader::endian(Cursor::new(&expected_bytes), LittleEndian);
            let mut e_recorder = BitRecorder::new(&mut e_reader);
            let _ = Item::from_reader_with_context(&mut e_recorder, huffman, None, false);
            let expected_bits: Vec<bool> =
                e_recorder.recorded_bits.iter().map(|rb| rb.bit).collect();

            // 4. Calculate similarity
            let matches = actual_bits
                .iter()
                .zip(expected_bits.iter())
                .filter(|(a, b)| a == b)
                .count();
            let max_len = actual_bits.len().max(expected_bits.len());
            let sim = if max_len > 0 {
                (matches as f64 / max_len as f64) * 100.0
            } else {
                0.0
            };
            (sim, item.code.clone())
        }
        Err(_) => (0.0, "FAIL".to_string()),
    }
}

// Re-using common logic from SymmetryBitDiff
fn load_items_with_recorder(bytes: &[u8], huffman: &HuffmanTree) -> Vec<(Item, Vec<bool>)> {
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
                if b_off > 0 {
                    let _ = reader.skip(b_off).ok();
                }
                let mut recorder = BitRecorder::new(&mut reader);
                match Item::from_reader_with_context(&mut recorder, huffman, None, false) {
                    Ok(item) => {
                        let consumed = recorder.total_read;
                        let bits = recorder.recorded_bits.iter().map(|rb| rb.bit).collect();
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
        if !collection.is_empty() {
            break;
        }
    }
    collection
}
