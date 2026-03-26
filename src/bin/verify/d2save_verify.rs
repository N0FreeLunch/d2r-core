use std::env;
use std::fs;
use std::process;
use std::io::Cursor;
use bitstream_io::{BitReader, LittleEndian};

use d2r_core::save::{Save, class_name, find_jm_markers, recalculate_checksum};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2save_verify <file.d2s> [file2.d2s ...]");
        process::exit(1);
    }

    let mut all_ok = true;

    for path in &args[1..] {
        println!("=== {} ===", path);
        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  [ERROR] Cannot read file: {}", e);
                all_ok = false;
                continue;
            }
        };

        let save = match Save::from_bytes(&bytes) {
            Ok(save) => save,
            Err(err) => {
                println!("  [FAIL]  Header parse: {}", err);
                all_ok = false;
                println!();
                continue;
            }
        };

        // Item round-trip symmetry check
        let huffman = d2r_core::item::HuffmanTree::new();
        let alpha_mode = save.header.version == 105;
        let items = match d2r_core::item::Item::read_player_items(&bytes, &huffman, alpha_mode) {
            Ok(items) => items,
            Err(err) => {
                println!("  [FAIL]  Item parse: {}", err);
                all_ok = false;
                println!();
                continue;
            }
        };

        let mut all_items_symmetric = true;
        for item in &items {
            let item_bits = match item.to_bytes(&huffman, alpha_mode) {
                Ok(b) => b,
                Err(e) => {
                    println!("  [FAIL]  Item to_bytes ({}): {}", item.code, e);
                    all_items_symmetric = false;
                    continue;
                }
            };
            // Try to parse back
            if let Err(e) = d2r_core::item::Item::from_bytes(&item_bits, &huffman, alpha_mode) {
                println!("  [FAIL]  Item round-trip parse failure ({}): {}", item.code, e);
                all_items_symmetric = false;
            }
        }

        if all_items_symmetric {
            println!("  [OK]    Item round-trip symmetry confirmed ({} items).", items.len());
        } else {
            all_ok = false;
        }

        println!("  [OK]    Magic: 0x{:08X}", save.header.magic);
        println!(
            "  [INFO]  Character: '{}' / {} / level {} / version 0x{:04X}",
            save.header.char_name,
            class_name(save.header.char_class),
            save.header.char_level,
            save.header.version
        );

        let header_size = save.header.file_size as usize;
        let actual_size = bytes.len();
        if header_size != actual_size {
            println!(
                "  [FAIL]  File size header: {} bytes, actual: {} bytes",
                header_size, actual_size
            );
            all_ok = false;
        } else {
            println!(
                "  [OK]    File size: {} bytes (header matches actual)",
                actual_size
            );
        }

        let stored_checksum = save.header.checksum;
        let calculated_checksum = match recalculate_checksum(&bytes) {
            Ok(checksum) => checksum,
            Err(err) => {
                println!("  [FAIL]  Checksum recalculation: {}", err);
                all_ok = false;
                println!();
                continue;
            }
        };

        if stored_checksum != calculated_checksum {
            println!(
                "  [FAIL]  Checksum: stored=0x{:08X}, calculated=0x{:08X}",
                stored_checksum, calculated_checksum
            );
            all_ok = false;
        } else {
            println!("  [OK]    Checksum: 0x{:08X}", stored_checksum);
        }

        let jm_positions = find_jm_markers(&bytes);
        if jm_positions.is_empty() {
            println!("  [WARN]  No JM markers found");
        } else {
            let count_offset = jm_positions[0];
            let item_count = u16::from_le_bytes([bytes[count_offset + 2], bytes[count_offset + 3]]);
            println!("  [OK]    JM markers at bytes: {:?}", jm_positions);
            println!("  [OK]    Player item count: {}", item_count);

            let huffman = d2r_core::item::HuffmanTree::new();
            let scanned = d2r_core::item::Item::scan_items(&bytes, &huffman);
            println!(
                "  [INFO]  Scanned {} items via pattern match:",
                scanned.len()
            );
            for (bit_pos, code) in scanned.iter().take(20) {
                println!(
                    "    - Bit {:>5}: code '{}' (byte {}, bit offset {})",
                    bit_pos,
                    code,
                    bit_pos / 8,
                    bit_pos % 8
                );
            }
        }

        println!();
    }

    if !all_ok {
        process::exit(1);
    }
}
