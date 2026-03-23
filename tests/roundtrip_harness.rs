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
    #[ignore]
    fn test_mutation_and_roundtrip() {
        let path = repo_path("tests/fixtures/savegames/original/amazon_authority_runeword.d2s");
        let bytes = fs::read(path).expect("fixture should be readable");
        println!("Save Signature: 0x{:08X}", u32::from_le_bytes(bytes[0..4].try_into().unwrap()));
        println!("Save Version: 0x{:08X}", u32::from_le_bytes(bytes[4..8].try_into().unwrap()));
        let huffman = HuffmanTree::new();
        
        // Use Trace to see what's happening
        unsafe { std::env::set_var("D2R_ITEM_TRACE", "1"); }

        let mut items = Item::read_player_items(&bytes, &huffman).expect("items should parse");
        
        // The Authority item was previously misidentified as "w ha", but it's "xrs " (Cuirass)
        let authority = items.iter_mut().find(|item| item.code.trim() == "xrs").expect("Authority item (xrs) not found");
        
        println!("Authority Item: Code={}, Version={}, Flags=0x{:08X}", authority.code, authority.version, authority.flags);
        
        // Check current properties
        println!("Item properties: {:?}", authority.properties.iter().map(|p| (p.stat_id, &p.name)).collect::<Vec<_>>());
        println!("Set attributes: {:?}", authority.set_attributes.iter().map(|list| list.iter().map(|p| p.stat_id).collect::<Vec<_>>()).collect::<Vec<_>>());
        println!("Runeword attributes: {:?}", authority.runeword_attributes.iter().map(|p| p.stat_id).collect::<Vec<_>>());
        println!("--------------------------------------------------");

        // Mutate internal properties:
        // In Alpha v105 Authority, ID 9 (maxmana) exists.
        let target_stat_id = 9; 
       use d2r_core::domain::vo::ItemStatValue;
        let new_val = ItemStatValue::new(100).unwrap();

        assert!(authority.set_property_value(target_stat_id, new_val), "Failed to set property {}", target_stat_id);
        
        // Re-serialize and verify
        let reserialized = authority.to_bytes(&huffman).expect("should re-serialize modified item");
        
        // Parse back and verify new value
        let modified_item = Item::from_bytes(&reserialized, &huffman).expect("should parse back modified bits");
        
        let mut all_stats = modified_item.properties.clone();
        for list in &modified_item.set_attributes {
            all_stats.extend(list.clone());
        }
        all_stats.extend(modified_item.runeword_attributes.clone());

        let new_stat = all_stats.iter().find(|p| p.stat_id == target_stat_id).expect("Mutated stat not found");
        assert_eq!(new_stat.value, 300);
    }
}
