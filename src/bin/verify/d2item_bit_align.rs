use d2r_core::item::{HuffmanTree, Item, BitRecorder};
use d2r_core::algo::alignment::BitAligner;
use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::{self, Cursor};
use std::path::PathBuf;

fn main() -> io::Result<()> {
    let _ = dotenvy::dotenv();
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("CLI Usage: cargo run --bin d2item_bit_align -- <save_file_path> <item_index>");
        return Ok(());
    }

    let save_path = &args[1];
    let item_index: usize = args[2].parse().map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid item index"))?;

    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();
    
    println!("[d2item_bit_align] Scanning items...");
    let items = load_items_scanning(&bytes, &huffman);
    
    if item_index >= items.len() {
        println!("[d2item_bit_align] Error: Item index {} out of bounds. Found {} items.", item_index, items.len());
        if !items.is_empty() {
            println!("Available items:");
            for (i, it) in items.iter().enumerate() {
                println!("  #{}: {} (ver={}, loc={}, mode={})", i, it.code.trim(), it.version, it.location, it.mode);
            }
        }
        return Err(io::Error::new(io::ErrorKind::NotFound, "Item index out of range"));
    }

    let item = &items[item_index];
    let actual: Vec<bool> = item.bits.iter().map(|rb| rb.bit).collect();

    // Strategy A: Re-serialize and re-parse to get "Expected Bits"
    let mut expected_item = item.clone();
    expected_item.bits.clear(); // Force re-encoding
    let expected_encoded_bytes = expected_item.to_bytes(&huffman)?;
    
    let mut reader = BitReader::endian(Cursor::new(&expected_encoded_bytes), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    let _ = Item::from_reader_with_context(&mut recorder, &huffman, None).ok();
    let expected: Vec<bool> = recorder.recorded_bits.iter().map(|rb| rb.bit).collect();

    let aligner = BitAligner::new(2, -1, -3, -1);
    let result = aligner.align(&actual, &expected);

    println!("[d2item_bit_align] Save: {} | Item #{} ({})", 
        PathBuf::from(save_path).file_name().unwrap_or_default().to_string_lossy(),
        item_index,
        item.code.trim()
    );
    println!("  Actual  bits : {}", actual.len());
    println!("  Expected bits: {}", expected.len());
    println!("  Similarity   : {:.2}%", result.similarity_pct());
    println!("  Gap count    : {}", result.gap_indices.len());
    
    if result.similarity_pct() < 100.0 {
        println!("\nAlignment Visualization:");
        println!("{}", result.pretty_print());
    } else {
        println!("  Perfect match (100.00%)!");
    }

    Ok(())
}

fn load_items_scanning(bytes: &[u8], huffman: &HuffmanTree) -> Vec<Item> {
    let mut all_items = Vec::new();
    
    // Find JM item section
    let mut jm_pos = 0;
    while let Some(rel_jm) = bytes[jm_pos..].windows(2).position(|w| w == b"JM") {
        let abs_jm = jm_pos + rel_jm;
        if abs_jm + 4 <= bytes.len() {
             let count = u16::from_le_bytes([bytes[abs_jm + 2], bytes[abs_jm + 3]]);
             if count > 0 && count < 1000 {
                  scan_at_offset(&bytes[(abs_jm+4)..], huffman, &mut all_items);
             }
        }
        jm_pos = abs_jm + 2;
        if !all_items.is_empty() { break; }
    }
    all_items
}

fn scan_at_offset(bytes: &[u8], huffman: &HuffmanTree, collection: &mut Vec<Item>) {
    let mut bit_pos = 0u64;
    let bit_limit = bytes.len() as u64 * 8;
    
    while bit_pos < bit_limit - 64 {
        let b_start = (bit_pos / 8) as usize;
        let b_off = (bit_pos % 8) as u32;
        let mut cursor = Cursor::new(&bytes[b_start..]);
        let mut reader = BitReader::endian(&mut cursor, LittleEndian);
        if b_off > 0 { let _ = reader.skip(b_off).ok(); }
        
        let mut recorder = BitRecorder::new(&mut reader);
        match Item::from_reader_with_context(&mut recorder, huffman, None) {
            Ok(item) => {
                let consumed = reader.position_in_bits().unwrap_or(0);
                if consumed >= 60 {
                    bit_pos += consumed;
                    collection.push(item);
                } else {
                    bit_pos += 1;
                }
            }
            Err(_) => {
                bit_pos += 1;
            }
        }
    }
}
