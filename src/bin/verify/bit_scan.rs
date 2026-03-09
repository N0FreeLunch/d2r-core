use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).unwrap();
    let huffman = HuffmanTree::new();

    let starts = Item::scan_items(&bytes, &huffman);
    println!("Found {} items via scan:", starts.len());
    for (bit, code) in starts {
        println!("  Bit {}: code '{}'", bit, code);
    }
}
