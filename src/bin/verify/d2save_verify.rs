use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;
use serde::Serialize;

use d2r_core::save::{Save, class_name, find_jm_markers, recalculate_checksum};

#[derive(Serialize)]
struct VerifyIssue {
    kind: String,
    message: String,
}

#[derive(Serialize)]
struct VerifyResult {
    file: String,
    status: String,
    issues: Vec<VerifyIssue>,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2save_verify <file.d2s> [file2.d2s ...]");
        process::exit(1);
    }

    if args.contains(&"--dump-bits".to_string()) {
        let idx = args.iter().position(|r| r == "--dump-bits").unwrap();
        let start_bit: u64 = args[idx + 1].parse().unwrap();
        let count: u64 = args[idx + 2].parse().unwrap();
        let path = &args[1];
        let bytes = fs::read(path).unwrap();

        println!("Dumping {} bits starting at {}:", count, start_bit);
        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        reader.skip(start_bit as u32).unwrap();

        for i in 0..count {
            let bit = if reader.read_bit().unwrap() { '1' } else { '0' };
            print!("{}", bit);
            if (i + 1) % 8 == 0 {
                print!(" ");
            }
            if (i + 1) % 64 == 0 {
                println!();
            }
        }
        println!();
        process::exit(0);
    }

    if args.contains(&"--json".to_string()) {
        let path = match args.iter().skip(1).find(|a| !a.starts_with("--")) {
            Some(p) => p,
            None => {
                eprintln!("Error: No file provided for --json");
                process::exit(1);
            }
        };

        let mut issues = Vec::new();
        let mut fail = false;

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                issues.push(VerifyIssue {
                    kind: "io".to_string(),
                    message: format!("Cannot read file: {}", e),
                });
                let result = VerifyResult {
                    file: path.clone(),
                    status: "fail".to_string(),
                    issues,
                };
                println!("{}", serde_json::to_string(&result).unwrap());
                process::exit(1);
            }
        };

        let save = match Save::from_bytes(&bytes) {
            Ok(s) => s,
            Err(err) => {
                issues.push(VerifyIssue {
                    kind: "header_parse".to_string(),
                    message: format!("{}", err),
                });
                let result = VerifyResult {
                    file: path.clone(),
                    status: "fail".to_string(),
                    issues,
                };
                println!("{}", serde_json::to_string(&result).unwrap());
                process::exit(1);
            }
        };

        let huffman = d2r_core::item::HuffmanTree::new();
        let alpha_mode = save.header.version == 105;
        let items = match d2r_core::item::Item::read_player_items(&bytes, &huffman, alpha_mode) {
            Ok(items) => items,
            Err(err) => {
                issues.push(VerifyIssue {
                    kind: "item_parse".to_string(),
                    message: format!("{}", err),
                });
                fail = true;
                Vec::new()
            }
        };

        for item in &items {
            let item_bits = match item.to_bytes(&huffman, alpha_mode) {
                Ok(b) => b,
                Err(e) => {
                    issues.push(VerifyIssue {
                        kind: "item_parse".to_string(),
                        message: format!("Item to_bytes ({}): {}", item.code, e),
                    });
                    fail = true;
                    continue;
                }
            };
            if let Err(e) = d2r_core::item::Item::from_bytes(&item_bits, &huffman, alpha_mode) {
                issues.push(VerifyIssue {
                    kind: "item_parse".to_string(),
                    message: format!("Item round-trip parse failure ({}): {}", item.code, e),
                });
                fail = true;
            }
        }

        if save.header.file_size as usize != bytes.len() {
            issues.push(VerifyIssue {
                kind: "file_size".to_string(),
                message: format!(
                    "File size header: {} bytes, actual: {} bytes",
                    save.header.file_size,
                    bytes.len()
                ),
            });
            fail = true;
        }

        match recalculate_checksum(&bytes) {
            Ok(calculated_checksum) => {
                if save.header.checksum != calculated_checksum {
                    issues.push(VerifyIssue {
                        kind: "checksum".to_string(),
                        message: format!(
                            "stored=0x{:08X}, calculated=0x{:08X}",
                            save.header.checksum, calculated_checksum
                        ),
                    });
                    fail = true;
                }
            }
            Err(err) => {
                issues.push(VerifyIssue {
                    kind: "checksum".to_string(),
                    message: format!("recalculation error: {}", err),
                });
                fail = true;
            }
        }

        if find_jm_markers(&bytes).is_empty() {
            issues.push(VerifyIssue {
                kind: "jm_markers".to_string(),
                message: "No JM markers found".to_string(),
            });
        }

        let result = VerifyResult {
            file: path.clone(),
            status: if fail { "fail".to_string() } else { "ok".to_string() },
            issues,
        };
        println!("{}", serde_json::to_string(&result).unwrap());
        process::exit(if fail { 1 } else { 0 });
    }

    let mut all_ok = true;

    for path in &args[1..] {
        if path.starts_with("--") {
            continue;
        }
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

        for (i, item) in items.iter().enumerate() {
            let trimmed = item.code.trim();
            println!(
                "  [Item {:>2}] {:<4} ID={:?}, Qual={:?}, Compact={}, StatBits={}, Props={}",
                i,
                trimmed,
                item.id,
                item.quality,
                item.is_compact,
                item.bits.len(),
                item.properties.len()
            );
            for prop in &item.properties {
                println!(
                    "    - ID={:<3}, Val={:<5}, Name={}",
                    prop.stat_id, prop.value, prop.name
                );
            }
        }

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
                println!(
                    "  [FAIL]  Item round-trip parse failure ({}): {}",
                    item.code, e
                );
                all_items_symmetric = false;
            }
        }

        if all_items_symmetric {
            println!(
                "  [OK]    Item round-trip symmetry confirmed ({} items).",
                items.len()
            );
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
