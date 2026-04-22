use d2r_core::item::{Item, HuffmanTree, is_plausible_item_header};
use std::fs;

fn main() {
    let fixture_path = "tests/fixtures/savegames/original/amazon_10_scrolls.d2s";
    let bytes = fs::read(fixture_path).expect("Fixture not found");
    
    let jm_pos = (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .expect("JM header not found");
    
    let huffman = HuffmanTree::new();
    let section_bytes = &bytes[jm_pos + 4..];
    let limit = (section_bytes.len() * 8) as u64;
    
    println!("Scanning for item headers in section 0...");
    let mut last_found = 0;
    for bit_offset in 0..limit {
        if let Some((mode, location, x, code, flags, version, is_compact, header_bits, nudge)) = 
            d2r_core::item::peek_item_header_at(section_bytes, bit_offset, &huffman, true) {
            if is_plausible_item_header(mode, location, &code, flags, version, true) {
                if bit_offset >= last_found + 72 { // Avoid overlapping headers
                    println!("Found plausible header at bit {}: code='{}', compact={}, mode={}, loc={}", 
                        bit_offset, code, is_compact, mode, location);
                    last_found = bit_offset;
                }
            }
        }
    }
}
