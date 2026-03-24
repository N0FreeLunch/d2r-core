use d2r_core::item::{HuffmanTree, Item};
use d2r_core::algo::alignment::{BitAligner, MsaResult};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: d2item_msa_analyser <save> <idx1> <idx2> <idx3> ...");
        println!("Example: d2item_msa_analyser save.d2s 0 1 2");
        process::exit(1);
    }

    let save_path = &args[1];
    let indices: Vec<usize> = args[2..].iter()
        .map(|s| s.parse().expect("index must be a number"))
        .collect();

    let bytes = fs::read(save_path)?;
    let huffman = HuffmanTree::new();

    let version = if bytes.len() >= 8 {
        u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]))
    } else {
        0
    };
    let alpha_mode = version == 105;

    let items = match Item::read_player_items(&bytes, &huffman, alpha_mode) {
        Ok(items) => items,
        Err(e) => {
            eprintln!("Error reading items: {}", e);
            process::exit(1);
        }
    };

    let mut item_bits = Vec::new();
    let mut item_info = Vec::new();

    for &idx in &indices {
        if idx >= items.len() {
            eprintln!("Error: Item index {} out of range (found {} items)", idx, items.len());
            process::exit(1);
        }
        let item = &items[idx];
        let bits: Vec<bool> = item.bits.iter().map(|rb| rb.bit).collect();
        item_bits.push(bits);
        item_info.push(format!("{} (#{})", item.code.trim(), idx));
    }

    let aligner = BitAligner::new(2, -1, -3, -1);
    let msa = aligner.msa(&item_bits);

    println!("--- Multiple Sequence Alignment (MSA) ---");
    println!("File  : {}", save_path);
    println!("Items : {}", item_info.join(", "));
    println!("-------------------------------------------");
    
    // Print aligned rows with info
    for (i, row) in msa.rows.iter().enumerate() {
        print!("{: <10}: ", item_info[i]);
        for bit in row {
            match bit {
                Some(true) => print!("1"),
                Some(false) => print!("0"),
                None => print!("-"),
            }
        }
        println!();
    }

    println!("-------------------------------------------");
    print!("{: <10}: ", "CONSENSUS");
    let consensus = msa.consensus();
    let mut conserved_count = 0;
    for bit in &consensus {
        match bit {
            Some(true) => {
                print!("1");
                conserved_count += 1;
            },
            Some(false) => {
                print!("0");
                conserved_count += 1;
            },
            None => print!("."),
        }
    }
    println!();
    println!("-------------------------------------------");
    
    let confidence = (conserved_count as f64 / consensus.len() as f64) * 100.0;
    println!("Conserved bits : {} / {}", conserved_count, consensus.len());
    println!("Confidence     : {:.2}%", confidence);

    Ok(())
}
