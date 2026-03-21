use std::env;
use std::fs;
use std::process;

use d2r_core::save::{
    ACTIVE_WEAPON_OFFSET, CHAR_CLASS_OFFSET, CHAR_LEVEL_OFFSET, CHAR_NAME_OFFSET,
    LAST_PLAYED_OFFSET, Save, class_name, find_jm_markers,
};

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

    let save = match Save::from_bytes(&bytes) {
        Ok(save) => save,
        Err(err) => {
            eprintln!("[ERROR] Cannot parse D2R header: {}", err);
            process::exit(1);
        }
    };

    println!();
    println!("[HEADER]");
    println!("  Offset  0 | Magic:         0x{:08X}", save.header.magic);
    println!("  Offset  4 | Version:       {}", save.header.version);
    println!(
        "  Offset  8 | File Size:     {} bytes",
        save.header.file_size
    );
    println!(
        "  Offset 12 | Checksum:      0x{:08X}",
        save.header.checksum
    );
    println!(
        "  Offset {ACTIVE_WEAPON_OFFSET:<2} | Active Weapon: {}",
        save.header.active_weapon
    );
    println!(
        "  Offset {CHAR_CLASS_OFFSET:<2} | Char Class:    {} ({})",
        class_name(save.header.char_class),
        save.header.char_class
    );
    println!(
        "  Offset {CHAR_LEVEL_OFFSET:<2} | Char Level:    {}",
        save.header.char_level
    );
    println!(
        "  Offset {LAST_PLAYED_OFFSET} | Last Played:   0x{:08X}",
        save.header.last_played
    );
    println!(
        "  Offset {CHAR_NAME_OFFSET} | Char Name:     '{}'",
        save.header.char_name
    );

    println!();
    println!("[JM MARKERS]");

    let jm_positions = find_jm_markers(&bytes);

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
