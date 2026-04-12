use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;
use serde::Serialize;

use d2r_core::save::{Save, class_name, find_jm_markers, recalculate_checksum};
use d2r_core::verify::{Report, ReportMetadata, ReportStatus, ReportIssue};
use d2r_core::verify::args::{ArgParser, ArgSpec};

#[derive(Serialize)]
struct D2SaveVerifyPayload {
    header_version: u32,
    alpha_mode: bool,
    file_size_header: usize,
    file_size_actual: usize,
    file_size_delta: i64,
    checksum_stored: String,
    checksum_calculated: Option<String>,
    jm_marker_count: usize,
    issue_count: usize,
}

fn main() {
    let mut parser = ArgParser::new("d2save_verify");
    parser.add_spec(
        ArgSpec::option("dump-bits", None, Some("dump-bits"), "Dump raw bits from start <bit> and count <bits>")
            .value_count(2)
    );
    parser.add_spec(ArgSpec::repeated_positional("files", "Save files to verify"));

    use d2r_core::verify::args::ArgError;
    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            process::exit(0);
        }
        Err(ArgError::Error(e)) => {
            eprintln!("error: {}", e);
            eprintln!("\n{}", parser.usage());
            process::exit(1);
        }
    };

    let files = parsed.get_vec("files").cloned().unwrap_or_default();
    let is_json = parsed.is_set("json");
    let dump_bits = parsed.get_vec("dump-bits");

    if let Some(bits_args) = dump_bits {
        if files.is_empty() {
            eprintln!("Error: No file provided for --dump-bits");
            process::exit(1);
        }
        let start_bit: u64 = bits_args[0].parse().unwrap_or(0);
        let count: u64 = bits_args[1].parse().unwrap_or(0);
        let path = &files[0];
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

    if is_json {
        if files.is_empty() {
            eprintln!("Error: No file provided for --json");
            process::exit(1);
        }
        let path = &files[0];

        let mut issues = Vec::new();
        let mut fail = false;

        let bytes = match fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                issues.push(ReportIssue {
                    kind: "io".to_string(),
                    message: format!("Cannot read file: {}", e),
                    bit_offset: None,
                });
                let result = Report::<serde_json::Value>::new(
                    ReportMetadata::new("d2save_verify", path, "unknown"),
                    ReportStatus::Fail,
                )
                .with_issues(issues)
                .with_hints(vec!["Ensure the file path is correct and accessible.".to_string()]);

                println!("{}", serde_json::to_string(&result).unwrap());
                process::exit(1);
            }
        };

        let save = match Save::from_bytes(&bytes) {
            Ok(s) => s,
            Err(err) => {
                issues.push(ReportIssue {
                    kind: "header_parse".to_string(),
                    message: format!("Header parse: {}", err),
                    bit_offset: None,
                });
                let result = Report::<serde_json::Value>::new(
                    ReportMetadata::new("d2save_verify", path, "corrupted"),
                    ReportStatus::Fail,
                )
                .with_issues(issues)
                .with_hints(vec!["Header is corrupted or in an unsupported format.".to_string()])
                .with_results(serde_json::json!({
                    "file_size_actual": bytes.len(),
                }));

                println!("{}", serde_json::to_string(&result).unwrap());
                process::exit(1);
            }
        };

        let huffman = d2r_core::item::HuffmanTree::new();
        let alpha_mode = save.header.version == 105;
        let items = match d2r_core::item::Item::read_player_items(&bytes, &huffman, alpha_mode) {
            Ok(items) => items,
            Err(err) => {
                let bit_offset = match err.error {
                    d2r_core::item::ParsingError::InvalidHuffmanBit { bit_offset } => Some(bit_offset),
                    d2r_core::item::ParsingError::InvalidStatId { bit_offset, .. } => Some(bit_offset),
                    d2r_core::item::ParsingError::UnexpectedSegmentEnd { bit_offset } => Some(bit_offset),
                    d2r_core::item::ParsingError::BitSymmetryFailure { bit_offset } => Some(bit_offset),
                    _ => None,
                };
                issues.push(ReportIssue {
                    kind: "item_parse".to_string(),
                    message: format!("{}", err),
                    bit_offset,
                });
                fail = true;
                Vec::new()
            }
        };

        for item in &items {
            let item_bits = match item.to_bytes(&huffman, alpha_mode) {
                Ok(b) => b,
                Err(e) => {
                    issues.push(ReportIssue {
                        kind: "item_parse".to_string(),
                        message: format!("Item to_bytes ({}): {}", item.code, e),
                        bit_offset: None,
                    });
                    fail = true;
                    continue;
                }
            };
            if let Err(e) = d2r_core::item::Item::from_bytes(&item_bits, &huffman, alpha_mode) {
                let bit_offset = match e.error {
                    d2r_core::item::ParsingError::InvalidHuffmanBit { bit_offset } => Some(bit_offset),
                    d2r_core::item::ParsingError::InvalidStatId { bit_offset, .. } => Some(bit_offset),
                    d2r_core::item::ParsingError::UnexpectedSegmentEnd { bit_offset } => Some(bit_offset),
                    d2r_core::item::ParsingError::BitSymmetryFailure { bit_offset } => Some(bit_offset),
                    _ => None,
                };
                issues.push(ReportIssue {
                    kind: "item_parse".to_string(),
                    message: format!("Item round-trip parse failure ({}): {}", item.code, e),
                    bit_offset,
                });
                fail = true;
            }
        }

        let header_size = save.header.file_size as usize;
        let actual_size = bytes.len();
        if header_size != actual_size {
            issues.push(ReportIssue {
                kind: "file_size".to_string(),
                message: format!(
                    "File size header: {} bytes, actual: {} bytes",
                    header_size, actual_size
                ),
                bit_offset: None,
            });
            fail = true;
        }

        let stored_checksum = save.header.checksum;
        let mut calculated_checksum_opt = None;
        match recalculate_checksum(&bytes) {
            Ok(calculated_checksum) => {
                calculated_checksum_opt = Some(calculated_checksum);
                if stored_checksum != calculated_checksum {
                    issues.push(ReportIssue {
                        kind: "checksum".to_string(),
                        message: format!(
                            "stored=0x{:08X}, calculated=0x{:08X}",
                            stored_checksum, calculated_checksum
                        ),
                        bit_offset: None,
                    });
                    fail = true;
                }
            }
            Err(err) => {
                issues.push(ReportIssue {
                    kind: "checksum".to_string(),
                    message: format!("recalculation error: {}", err),
                    bit_offset: None,
                });
                fail = true;
            }
        }

        let jm_markers = find_jm_markers(&bytes);
        if jm_markers.is_empty() {
            issues.push(ReportIssue {
                kind: "jm_markers".to_string(),
                message: "No JM markers found".to_string(),
                bit_offset: None,
            });
        }

        // Hint synthesis
        let mut hints = Vec::new();
        for issue in &issues {
            match issue.kind.as_str() {
                "io" => hints.push("Ensure the file path is correct and accessible.".to_string()),
                "header_parse" => hints.push("Header is corrupted or in an unsupported format.".to_string()),
                "item_parse" => {
                    if let Some(offset) = issue.bit_offset {
                        hints.push(format!("Investigate bit-width or alignment logic near bit offset {}.", offset));
                    } else {
                        hints.push("Check item data structure or Huffman encoding table.".to_string());
                    }
                },
                "file_size" => hints.push("File size in header must match the actual byte count. Truncation suspected.".to_string()),
                "checksum" => hints.push("Checksum must be refreshed after any file mutation (lives at offset 12).".to_string()),
                "jm_markers" => hints.push("Missing JM markers suggest the file is not a valid character save or is severely truncated.".to_string()),
                _ => {}
            }
        }
        hints.dedup();

        let issue_count = issues.len();
        let status = if fail { ReportStatus::Fail } else { ReportStatus::Ok };
        let version = format!("0x{:04X}", save.header.version);
        let result = Report::<D2SaveVerifyPayload>::new(
            ReportMetadata::new("d2save_verify", path, &version),
            status,
        )
        .with_issues(issues)
        .with_hints(hints)
        .with_results(D2SaveVerifyPayload {
            header_version: save.header.version,
            alpha_mode,
            file_size_header: header_size,
            file_size_actual: actual_size,
            file_size_delta: (actual_size as i64) - (header_size as i64),
            checksum_stored: format!("0x{:08X}", stored_checksum),
            checksum_calculated: calculated_checksum_opt.map(|c| format!("0x{:08X}", c)),
            jm_marker_count: jm_markers.len(),
            issue_count,
        });
        println!("{}", serde_json::to_string(&result).unwrap());
        process::exit(if fail { 1 } else { 0 });
    }

    if files.is_empty() {
        eprintln!("{}", parser.usage());
        process::exit(1);
    }

    let mut all_ok = true;

    for path in &files {
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
            let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];
            let scanned = d2r_core::item::Item::read_player_items(&bytes, &huffman, is_alpha).unwrap_or_default();
            println!(
                "  [INFO]  Parsed {} player items:",
                scanned.len()
            );
            for (idx, item) in scanned.iter().take(20).enumerate() {
                println!(
                    "    - Item {:>2}: code '{}' (start bit {}, bits {})",
                    idx,
                    item.code,
                    item.range.start,
                    item.total_bits
                );
            }
        }

        println!();
    }

    if !all_ok {
        process::exit(1);
    }
}
