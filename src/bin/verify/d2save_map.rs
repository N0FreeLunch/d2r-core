use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2save_map <file.d2s>");
        process::exit(1);
    }

    let path = &args[1];
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[ERROR] Cannot read '{}': {}", path, e);
            process::exit(1);
        }
    };

    println!("=== Section Map: {} ({} bytes) ===", path, bytes.len());

    // Header fields
    let magic = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let file_size = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let checksum = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let active_weapon = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);

    println!();
    println!("[HEADER]");
    println!("  Offset  0 | Magic:         0x{:08X}", magic);
    println!("  Offset  4 | Version:       {}", version);
    println!("  Offset  8 | File Size:     {} bytes", file_size);
    println!("  Offset 12 | Checksum:      0x{:08X}", checksum);
    println!("  Offset 16 | Active Weapon: {}", active_weapon);

    // Character name (offset 20, 16 bytes)
    let name_bytes = &bytes[20..36];
    let name = String::from_utf8_lossy(name_bytes)
        .trim_matches('\0')
        .to_string();
    println!("  Offset 20 | Char Name:     '{}'", name);

    // Char class (offset 40)
    let char_class = bytes[40];
    let class_name = match char_class {
        0 => "Amazon",
        1 => "Sorceress",
        2 => "Necromancer",
        3 => "Paladin",
        4 => "Barbarian",
        5 => "Druid",
        6 => "Assassin",
        _ => "Unknown",
    };
    println!(
        "  Offset 40 | Char Class:    {} ({})",
        class_name, char_class
    );

    println!();
    println!("[JM MARKERS]");

    // Find all JM markers
    let mut jm_positions: Vec<usize> = Vec::new();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'J' && bytes[i + 1] == b'M' {
            jm_positions.push(i);
        }
    }

    let section_labels = [
        "Player Items",
        "Corpse Items",
        "Mercenary Items",
        "Iron Golem",
    ];

    for (idx, &pos) in jm_positions.iter().enumerate() {
        let label = section_labels
            .get(idx)
            .copied()
            .unwrap_or("Unknown Section");
        let item_count = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
        let next_pos = jm_positions.get(idx + 1).copied().unwrap_or(bytes.len());
        let section_size = next_pos - pos;

        println!(
            "  [JM #{idx}] Offset {pos:>5} (bit {:>6}) | {label:<20} | count={item_count}, section_bytes={section_size}",
            pos * 8
        );
    }

    if jm_positions.is_empty() {
        println!("  [WARN] No JM markers found in file");
    }

    println!();
    println!("[SUMMARY]");
    println!("  Total JM sections: {}", jm_positions.len());
    if let Some(&first_jm) = jm_positions.first() {
        println!("  Header + pre-item data: {} bytes", first_jm);
    }
    if let (Some(&first_jm), Some(&second_jm)) = (jm_positions.get(0), jm_positions.get(1)) {
        let player_section_bytes = second_jm - first_jm;
        println!("  Player item section:    {} bytes", player_section_bytes);
    }
}
