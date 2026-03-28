#[cfg(test)]
mod tests {
    use d2r_core::item::{Item, HuffmanTree};
    use std::fs;

    #[test]
    fn test_alpha_v105_amazon_recovery_100pct() {
        let fixture_path = "tests/fixtures/savegames/original/amazon_10_scrolls.d2s";
        let bytes = fs::read(fixture_path).expect("Fixture not found at tests/fixtures/savegames/original/amazon_10_scrolls.d2s");
        
        let jm_pos = (0..bytes.len().saturating_sub(1))
            .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
            .expect("JM header not found");
        
        let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
        println!("Expected items: {}", count);
        
        let huffman = HuffmanTree::new();
        // Alpha mode = true
        let items = Item::read_player_items(&bytes[jm_pos..], &huffman, true).expect("Parsing failed");
        
        println!("Items recovered: {}", items.len());
        let mut recovered_codes = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let trimmed = item.code.trim().to_string();
            println!("[{:2}] {:<4} (id: {:?}, qual: {:?})", i, trimmed, item.id, item.quality);
            recovered_codes.push(trimmed);
        }
        
        assert_eq!(items.len() as u16, count, "Should recover all identified items (16)");
        
        // Verify specifically jav (14) and buc (15) are found in the sequence.
        assert!(recovered_codes.contains(&"jav".to_string()), "Javelin should be found");
        assert!(recovered_codes.contains(&"buc".to_string()), "Buckler should be found");
        
        // Final assertion: we have exactly 16 items.
        assert_eq!(items.len(), 16);
    }
}
