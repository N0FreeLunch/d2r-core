use d2r_core::item::{HuffmanTree, Item, ItemQuality};
use std::env;
use std::fs;
use std::io;
use bitstream_io::BitRead;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin d2item_chunk_verify -- <file.d2s> [--range START..END] [--detail INDEX]");
        std::process::exit(1);
    }

    let path = &args[1];
    let mut range_start = 0;
    let mut range_end = 10;
    let mut detail_index: Option<usize> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--range" if i + 1 < args.len() => {
                let parts: Vec<&str> = args[i + 1].split("..").collect();
                if parts.len() == 2 {
                    range_start = parts[0].parse().unwrap_or(0);
                    range_end = parts[1].parse().unwrap_or(10);
                }
                i += 2;
            }
            "--detail" if i + 1 < args.len() => {
                detail_index = Some(args[i + 1].parse().unwrap_or(0));
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let bytes = fs::read(path)?;
    let huffman = HuffmanTree::new();
    
    // Find all JM markers
    let mut jm_positions = Vec::new();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'J' && bytes[i + 1] == b'M' {
            jm_positions.push(i);
        }
    }

    if jm_positions.is_empty() {
        println!("No JM markers found.");
        return Ok(());
    }

    let mut all_items = Vec::new();
    for (jm_idx, &start_pos) in jm_positions.iter().enumerate() {
        let count_val = if start_pos + 3 < bytes.len() {
            u16::from_le_bytes([bytes[start_pos + 2], bytes[start_pos + 3]])
        } else {
            0
        };
        
        // Find next JM marker or end of file
        let end_marker = if jm_idx + 1 < jm_positions.len() {
            jm_positions[jm_idx + 1]
        } else {
            bytes.len()
        };
        
        println!("JM Section at 0x{:04X}: {} items", start_pos, count_val);
        if count_val > 0 {
            let section_data = &bytes[start_pos + 4..end_marker];
            
            if env::var("D2R_ITEM_TRACE").is_ok() {
                println!("  [Diagnostic] Performing bit-level probe for JM section at 0x{:04X}...", start_pos);
                let section_bits = (section_data.len() * 8) as u64;
                for b in 0..section_bits.saturating_sub(64) {
                    if is_terminator_like(section_data, b) {
                        println!("  [Probe] Possible Terminator at bit {}", b);
                        dump_bit_window(section_data, b);
                    }
                    let code = peek_code_minimal(section_data, b, &huffman);
                    if let Some(c) = code {
                        // Only dump window for plausible codes in the interesting region
                        if b >= 1000 && b <= 2000 {
                            println!("  [Probe] Plausible Header at bit {} (Code: '{}')", b, c);
                            dump_bit_window(section_data, b);
                        }
                    } else if b == 1127 {
                        // Explicitly requested bit window for bit 1127
                        println!("  [Probe] Target Diagnostic at bit 1127");
                        dump_bit_window(section_data, b);
                    }
                }
            }

            let result = Item::read_section(section_data, count_val, &huffman);
            match result {
                Ok(sect_items) => all_items.extend(sect_items),
                Err(err) => {
                    println!("  └── [ERROR] JM @ 0x{:04X}: {}", start_pos, err);
                    // Crucial: continue to collect items found before the error if possible
                    // But read_section currently returns Err and discards items.
                }
            }
        }
    }
    
    let items = all_items;
    
    if let Some(idx) = detail_index {
        if idx >= items.len() {
            eprintln!("Index {} out of bounds (total items: {})", idx, items.len());
            std::process::exit(1);
        }
        print_detail(idx, &items[idx]);
    } else {
        print_summary(&items, range_start, range_end);
    }

    Ok(())
}

fn print_summary(items: &[Item], start: usize, end: usize) {
    let actual_end = end.min(items.len());
    println!("Total Items Found: {}", items.len());
    println!("Scanning Range: {}..{}", start, actual_end);
    println!();
    println!("{:>5} | {:<5} | {:>4} | {:<10} | {:<4} | {:<8}", "Index", "Code", "Bits", "Quality", "RW", "Loc");
    println!("------|-------|------|------------|------|---------");

    for i in start..actual_end {
        let item = &items[i];
        let quality_str = match item.quality {
            Some(ItemQuality::Normal) => "Normal",
            Some(ItemQuality::Magic) => "Magic",
            Some(ItemQuality::Set) => "Set",
            Some(ItemQuality::Unique) => "Unique",
            Some(ItemQuality::Rare) => "Rare",
            Some(ItemQuality::Crafted) => "Crafted",
            _ => "Other",
        };

        println!(
            "{:>5} | {:<5} | {:>4} | {:<10} | {:<4} | G:{:<1} P:{:<1} L:{:<2} S:{:<1}",
            i,
            item.code,
            item.bits.len(),
            quality_str,
            if item.is_runeword { "YES" } else { "NO" },
            item.mode,
            item.page,
            item.location,
            item.socketed_items.len()
        );
        
        for child in &item.socketed_items {
            println!("      └── Socketed: '{}' ({} bits)", child.code, child.bits.len());
        }
        
        // Basic anomaly check
        if item.is_runeword && item.bits.len() < 100 {
            println!("      └── [WARN] Runeword with suspicious short bit-length: {}", item.bits.len());
        }
        if item.quality == Some(ItemQuality::Normal) && item.bits.len() > 200 {
            println!("      └── [WARN] Normal item with suspicious long bit-length: {}", item.bits.len());
        }
    }
}

fn print_detail(index: usize, item: &Item) {
    println!("=== Detail View: Item Index {} ===", index);
    println!("Code: '{}'", item.code);
    println!("Bits Length: {}", item.bits.len());
    println!("Flags: 0x{:08X}", item.flags);
    println!("Version: {}", item.version);
    println!("Socketed: {}", (item.flags & (1 << 11)) != 0);
    println!("Quality: {:?}", item.quality);
    println!("Runeword: {}", item.is_runeword);
    println!("Location: Mode={} Page={} X={} Y={} Loc={}", item.mode, item.page, item.x, item.y, item.location);
    println!("Properties Complete: {}", item.properties_complete);
    println!();
    println!("Properties:");
    for prop in &item.properties {
        println!("  ID {:>3}: Value {}", prop.stat_id, prop.value);
    }
    if !item.runeword_attributes.is_empty() {
        println!("Runeword Attributes:");
        for prop in &item.runeword_attributes {
            println!("  ID {:>3}: Value {}", prop.stat_id, prop.value);
        }
    }
}

fn dump_bit_window(data: &[u8], pos: u64) {
    let half = 32;
    let start = pos.saturating_sub(half);
    let end = (pos + half).min((data.len() * 8) as u64);
    
    print!("  [BitWindow] @{:>4}: ", pos);
    for b in start..end {
        let byte_idx = (b / 8) as usize;
        let bit_idx = (b % 8) as u8;
        let bit = (data[byte_idx] >> bit_idx) & 1 != 0;
        print!("{}", if bit { "1" } else { "0" });
        if b % 8 == 7 { print!(" "); }
    }
    println!();
}

fn is_terminator_like(data: &[u8], bit_pos: u64) -> bool {
    // Look for 9 ones followed by 8 zeros (17 bits)
    let mut reader = bitstream_io::BitReader::endian(std::io::Cursor::new(data), bitstream_io::LittleEndian);
    if reader.skip(bit_pos as u32).is_err() { return false; }
    
    let mut val = 0u32;
    for i in 0..17 {
        if let Ok(bit) = reader.read_bit() {
            if bit {
                val |= 1 << i;
            }
        } else {
            return false;
        }
    }
    val == 0x1FF // Exactly FF 01 00 in little-endian bytes starting at bit_pos
}

fn peek_code_minimal(data: &[u8], start_bit: u64, huffman: &HuffmanTree) -> Option<String> {
    // Flags(32)+Ver(3)+Mode(3)+Loc(4)+X(4) = 46 bits
    // We'll just try decoding at offsets 46, 46+7 (if Loc 0), etc.
    for offset in [46u64, 46+7] { 
        let mut reader = bitstream_io::BitReader::endian(std::io::Cursor::new(data), bitstream_io::LittleEndian);
        if reader.skip((start_bit + offset) as u32).is_err() { continue; }
        let mut code = String::new();
        let mut ok = true;
        for _ in 0..4 {
            if let Ok(c) = huffman.decode(&mut reader) {
                code.push(c);
            } else {
                ok = false;
                break;
            }
        }
        if ok {
            let trimmed = code.trim();
            if trimmed.len() >= 3 && trimmed.chars().all(|c| c.is_alphanumeric()) {
                let known = ["jav", "buc", "rin", "amu", "key", "tsc", "isc", "hp1", "mp1"];
                if known.contains(&trimmed) {
                    return Some(code);
                }
            }
        }
    }
    None
}
