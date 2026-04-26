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

    #[test]
    fn test_all_alpha_v105_fixtures_bit_perfect() {
        let fixtures = [
            "tests/fixtures/savegames/original/amazon_authority_runeword.d2s",
            "tests/fixtures/savegames/original/amazon_10_scrolls.d2s",
            "tests/fixtures/savegames/original/amazon_v105_act2_start.d2s",
            "tests/fixtures/savegames/original/amazon_v105_andariel_killed_no_talk.d2s",
            "tests/fixtures/savegames/original/amazon_v105_re_probe_zigzag_all_diff.d2s",
        ];
        
        let huffman = HuffmanTree::new();
        
        for fixture_path in fixtures {
            println!("Testing Alpha v105 bit-perfect roundtrip for: {}", fixture_path);
            let bytes = fs::read(fixture_path).expect("Fixture not found");
            
            // 1. Recover all items
            let items = Item::read_player_items(&bytes, &huffman, true).expect("Parsing failed");
            
            // 2. Reserialize section
            let reserialized_items = Item::serialize_section(&items, &huffman, true).expect("Serialization failed");
            
            // 3. Compare with original bytes
            let jm_pos = (0..bytes.len().saturating_sub(1))
                .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
                .expect("JM header not found");
            
            let original_payload = &bytes[jm_pos + 4..];
            
            // The items section in original files might contain more data (other sections),
            // so we compare only up to the length of our reserialized bits.
            // But for these specific Alpha fixtures, we aim for 100% segment matching.
            for i in 0..reserialized_items.len() {
                assert_eq!(
                    reserialized_items[i], 
                    original_payload[i], 
                    "Byte mismatch at offset {} in fixture {}", i, fixture_path
                );
            }
            println!("  [PASS] {} bytes matched perfectly.", reserialized_items.len());
        }
    }
}
