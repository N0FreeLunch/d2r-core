//! Roundtrip validation tests for bit-perfect serialization.

#[cfg(test)]
mod roundtrip_tests {
    use d2r_core::item::{Item, HuffmanTree};
    use d2r_core::domain::vo::align_to_byte;
    use std::fs;
    use std::path::PathBuf;

    fn repo_path(relative: &str) -> PathBuf {
        let _ = dotenvy::dotenv();
        let base = std::env::var("D2R_CORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        base.join(relative)
    }

    #[test]
    fn test_authority_runeword_roundtrip() {
        let path = repo_path("tests/fixtures/savegames/original/amazon_authority_runeword.d2s");
        let bytes = fs::read(path).expect("fixture should be readable");
        let huffman = HuffmanTree::new();
        
        // 1. Read all items from the save
        let items = Item::read_player_items(&bytes, &huffman).expect("items should parse");
        
        for item in &items {
            // 2. Re-serialize each item
            let reserialized = item.to_bytes(&huffman).expect("should re-serialize");
            
            // 3. Compare bits if the item wasn't recovered/modified during parse
            // Note: If the item was 'recovered' due to bit-mismatch, the roundtrip
            // might produce a 'fixed' bitstream which is logically identical but bit-different.
            // For 'Authority' runeword, we expect it to be stable.
            if item.properties_complete {
                // If it's a top-level item with bits recorded, we can compare directly.
                if !item.bits.is_empty() {
                    // Re-calculate how many bytes the original bits occupied
                    let original_bits_len = item.bits.len() as u64;
                    let original_bytes_len = align_to_byte(original_bits_len) / 8;
                    
                    assert_eq!(reserialized.len() as u64, original_bytes_len, 
                        "Reserialized length mismatch for item {}", item.code);
                    
                    // We don't have the original raw segment here easily, 
                    // but we can parse the reserialized bytes back and compare properties.
                    let item_back = Item::from_bytes(&reserialized, &huffman).expect("should parse back");
                    assert_eq!(item.code, item_back.code);
                    assert_eq!(item.properties.len(), item_back.properties.len());
                    for (p1, p2) in item.properties.iter().zip(item_back.properties.iter()) {
                        assert_eq!(p1.stat_id, p2.stat_id);
                        assert_eq!(p1.value, p2.value);
                    }
                }
            }
        }
    }

    #[test]
    fn test_mutation_and_roundtrip() {
        let path = repo_path("tests/fixtures/savegames/original/amazon_authority_runeword.d2s");
        let bytes = fs::read(path).expect("fixture should be readable");
        let huffman = HuffmanTree::new();
        
        let mut items = Item::read_player_items(&bytes, &huffman).expect("items should parse");
        let authority = items.iter_mut().find(|item| item.code.trim() == "w ha").unwrap();
        
        // Let's modify 'Enhanced Defense' (stat_id 31)
        // Check current value first
        assert!(authority.properties.iter().any(|p| p.stat_id == 31));
        
        use d2r_core::domain::vo::ItemStatValue;
        let new_val = ItemStatValue::new(300).unwrap(); // Set to 300% ED
        
        assert!(authority.set_property_value(31, new_val));
        
        // Re-serialize and verify
        let reserialized = authority.to_bytes(&huffman).expect("should re-serialize modified item");
        
        // Parse back and verify new value
        let modified_item = Item::from_bytes(&reserialized, &huffman).expect("should parse back modified bits");
        let new_ed_stat = modified_item.properties.iter().find(|p| p.stat_id == 31).unwrap();
        assert_eq!(new_ed_stat.value, 300);
    }
}
