use d2r_core::item::{HuffmanTree, Item};
use std::fs;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let huffman = HuffmanTree::new();
    
    // We want to debug why Authority (Index 5) is failing or correctly identified.
    // Index 5 starts at bit 7744.
    
    match Item::read_player_items(&bytes, &huffman) {
        Ok(items) => {
            println!("Parsed {} items.", items.len());
            for (i, item) in items.iter().enumerate() {
                println!("Item {}: code={}, bin_len={} bits", i, item.code, item.bits.len());
            }
        }
        Err(e) => {
            println!("Error parsing items: {}", e);
        }
    }
}
