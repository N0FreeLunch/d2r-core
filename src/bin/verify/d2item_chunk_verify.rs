use d2r_core::item::{HuffmanTree, Item, ItemQuality};
use std::env;
use std::fs;
use std::io;

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
            let result = Item::read_section(section_data, count_val, &huffman);
            match result {
                Ok(sect_items) => all_items.extend(sect_items),
                Err(err) => println!("  └── [ERROR] JM @ 0x{:04X}: {}", start_pos, err),
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
