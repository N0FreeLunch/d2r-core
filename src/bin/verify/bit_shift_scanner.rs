use d2r_core::item::{HuffmanTree, Item, BitRecorder, ParsingResult};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use serde::Serialize;
use std::env;
use std::fs;
use std::io::{self, Cursor};
use std::path::PathBuf;

#[derive(Serialize, Debug)]
struct BitStep {
    relative_offset: i64,
    score: f64,
    code: String,
    consumed_bits: u64,
    is_valid: bool,
    error: Option<String>,
}

#[derive(Serialize, Debug)]
struct ScanResult {
    file_path: String,
    base_byte_offset: usize,
    window_bits: isize,
    candidates: Vec<BitStep>,
}

impl std::fmt::Display for BitStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let status = if self.is_valid { "OK " } else { "ERR" };
        write!(f, "[Offset {:+4}] [{}] {} (bits={}) score={:.2}", 
            self.relative_offset, status, self.code.trim(), self.consumed_bits, self.score)
    }
}

fn main() -> io::Result<()> {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    
    let mut save_path_str = None;
    let mut byte_offset = None;
    let mut window = 128;
    let mut use_json = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--json" => use_json = true,
            "--window" => {
                i += 1;
                if i < args.len() {
                    window = args[i].parse().unwrap_or(128);
                }
            }
            _ => {
                if save_path_str.is_none() {
                    save_path_str = Some(&args[i]);
                } else if byte_offset.is_none() {
                    byte_offset = Some(usize::from_str_radix(args[i].trim_start_matches("0x"), 16).unwrap_or(0));
                }
            }
        }
        i += 1;
    }

    if save_path_str.is_none() || byte_offset.is_none() {
        println!("CLI Usage: cargo run --bin BitShiftScanner -- <save_file> <byte_offset_hex> [--window <bits>] [--json]");
        return Ok(());
    }

    let save_path = save_path_str.unwrap();
    let byte_offset = byte_offset.unwrap();
    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();

    let mut result = run_bit_scan(&bytes, byte_offset, window, &huffman);
    result.file_path = save_path.clone();

    if use_json {
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    } else {
        println!("[BitShiftScanner] File: {}", PathBuf::from(save_path).file_name().unwrap_or_default().to_string_lossy());
        println!("  Base Offset: 0x{:04X}", byte_offset);
        println!("  Window     : ±{} bits", window);
        println!("  Candidates Found: {}", result.candidates.len());
        println!("--------------------------------------------------");
        for (idx, cand) in result.candidates.iter().enumerate().take(20) {
            println!("  #{:02}: {}", idx, cand);
        }
    }

    // Always save trace to tmp/
    let _ = fs::create_dir_all("tmp");
    let trace_path = "tmp/bit_scan_report.json";
    let _ = fs::write(trace_path, serde_json::to_string_pretty(&result).unwrap());

    Ok(())
}

fn run_bit_scan(bytes: &[u8], base_byte_offset: usize, window: isize, huffman: &HuffmanTree) -> ScanResult {
    let mut candidates = Vec::new();
    
    let base_bit_pos = (base_byte_offset as u64) * 8;
    let start_bit = if base_bit_pos > window as u64 { base_bit_pos - window as u64 } else { 0 };
    let end_bit = base_bit_pos + window as u64;

    for bit_pos in start_bit..=end_bit {
        let b_start = (bit_pos / 8) as usize;
        let b_off = (bit_pos % 8) as u32;
        
        if b_start + 4 > bytes.len() { break; }

        let mut cursor = Cursor::new(&bytes[b_start..]);
        let mut reader = BitReader::endian(&mut cursor, LittleEndian);
        if b_off > 0 { let _ = reader.skip(b_off).ok(); }
        
        let mut recorder = BitRecorder::new(&mut reader);
        let parse_res = Item::from_reader_with_context(&mut recorder, huffman, None, false);
        
        let consumed = reader.position_in_bits().unwrap_or(0);
        let offset_diff = bit_pos as i64 - base_bit_pos as i64;

        let (is_valid, code, error) = match parse_res {
            Ok(it) => (true, it.code.clone(), None),
            Err(e) => (false, "ERR ".to_string(), Some(e.to_string())),
        };

        let mut score = 0.0;
        if is_valid { score += 50.0; }
        if consumed >= 32 && consumed <= 1000 { score += 30.0; }
        if !code.contains("ERR") && code.trim().len() == 4 { score += 20.0; }

        if consumed >= 16 {
            candidates.push(BitStep {
                relative_offset: offset_diff,
                score,
                code: code.clone(),
                consumed_bits: consumed,
                is_valid,
                error,
            });
        }
    }

    candidates.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    ScanResult {
        file_path: "".to_string(),
        base_byte_offset,
        window_bits: window,
        candidates,
    }
}
