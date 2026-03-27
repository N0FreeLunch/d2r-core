use d2r_core::item::{HuffmanTree, Item};
use std::fs;

fn main() {
    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s").unwrap();
    let huffman = HuffmanTree::new();

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    println!("File version: {}", version);

    match Item::read_player_items(&bytes, &huffman, version == 105) {
        Ok(items) => {
            println!("Parsed {} items.", items.len());
            for (i, item) in items.iter().enumerate() {
                println!(
                    "Item {:>2}: code={}, flags={:032b}, bin_len={} bits, properties={}",
                    i,
                    item.code.trim(),
                    item.flags,
                    item.bits.len(),
                    item.properties.len()
                );
                for (pi, prop) in item.properties.iter().enumerate() {
                    println!(
                        "   Prop {:>2}: id={}, name={}, val={}",
                        pi, prop.stat_id, prop.name, prop.value
                    );
                }
            }
        }
        Err(e) => {
            println!("Error parsing items: {}", e);
        }
    }
}
