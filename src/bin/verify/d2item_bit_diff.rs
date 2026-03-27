use d2r_core::algo::alignment::BitAligner;
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 5 {
        println!("Usage: d2item_bit_diff <save1> <index1> <save2> <index2>");
        println!("Example: d2item_bit_diff save1.d2s 0 save2.d2s 1");
        process::exit(1);
    }

    let save1_path = &args[1];
    let index1: usize = args[2].parse().expect("index1 must be a number");
    let save2_path = &args[3];
    let index2: usize = args[4].parse().expect("index2 must be a number");

    let bytes1 = fs::read(save1_path)?;
    let bytes2 = fs::read(save2_path)?;

    let huffman = HuffmanTree::new();

    let items1 = load_items(&bytes1, &huffman, save1_path);
    let items2 = load_items(&bytes2, &huffman, save2_path);

    if index1 >= items1.len() {
        eprintln!(
            "Error: Item index {} out of range for {} (found {} items)",
            index1,
            save1_path,
            items1.len()
        );
        process::exit(1);
    }
    if index2 >= items2.len() {
        eprintln!(
            "Error: Item index {} out of range for {} (found {} items)",
            index2,
            save2_path,
            items2.len()
        );
        process::exit(1);
    }

    let item1 = &items1[index1];
    let item2 = &items2[index2];

    let bits1: Vec<bool> = item1.bits.iter().map(|rb| rb.bit).collect();
    let bits2: Vec<bool> = item2.bits.iter().map(|rb| rb.bit).collect();

    let aligner = BitAligner::new(2, -1, -3, -1); // match, mismatch, gap_open, gap_extend
    let result = aligner.align(&bits1, &bits2);

    println!("--- Bitstream Alignment Diff ---");
    println!(
        "Item A: {} #{} ({})",
        Path::new(save1_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        index1,
        item1.code.trim()
    );
    println!(
        "Item B: {} #{} ({})",
        Path::new(save2_path)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        index2,
        item2.code.trim()
    );
    println!("--------------------------------");
    println!("Score        : {}", result.score);
    println!("Gap Count    : {}", result.gap_indices.len());
    println!("Similarity   : {:.2}%", result.similarity_pct());
    println!("--------------------------------");
    println!("{}", result.pretty_print());
    println!("--------------------------------");

    Ok(())
}

fn load_items(bytes: &[u8], huffman: &HuffmanTree, path: &str) -> Vec<Item> {
    let version = if bytes.len() >= 8 {
        u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    let alpha_mode = version == 105;

    match Item::read_player_items(bytes, huffman, alpha_mode) {
        Ok(items) => items,
        Err(e) => {
            eprintln!("Warning: Error reading items from {}: {}", path, e);
            Vec::new()
        }
    }
}
