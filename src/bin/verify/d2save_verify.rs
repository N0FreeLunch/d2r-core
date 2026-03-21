use std::env;
use std::fs;
use std::process;

fn calculate_checksum(bytes: &[u8]) -> u32 {
    let mut checksum = 0u32;
    for &b in bytes {
        checksum = checksum.wrapping_shl(1) | checksum.wrapping_shr(31);
        checksum = checksum.wrapping_add(b as u32);
    }
    checksum
}

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

        // Check magic bytes
        let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        if magic != 0xAA55AA55 {
            println!("  [WARN]  Magic: 0x{:08X} (expected 0xAA55AA55)", magic);
        } else {
            println!("  [OK]    Magic: 0x{:08X}", magic);
        }

        // Check file size in header (offset 8, 4 bytes)
        let header_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
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

        // Checksum (offset 12, 4 bytes)
        let stored_checksum = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);

        // Zero out checksum field before calculating
        let mut calc_bytes = bytes.clone();
        calc_bytes[12..16].copy_from_slice(&[0, 0, 0, 0]);
        let calculated_checksum = calculate_checksum(&calc_bytes);

        if stored_checksum != calculated_checksum {
            println!(
                "  [FAIL]  Checksum: stored=0x{:08X}, calculated=0x{:08X}",
                stored_checksum, calculated_checksum
            );
            all_ok = false;
        } else {
            println!("  [OK]    Checksum: 0x{:08X}", stored_checksum);
        }

        // JM markers
        let mut jm_positions: Vec<usize> = Vec::new();
        for i in 0..bytes.len().saturating_sub(1) {
            if bytes[i] == b'J' && bytes[i + 1] == b'M' {
                jm_positions.push(i);
            }
        }
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
