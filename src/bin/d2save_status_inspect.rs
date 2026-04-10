use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::save::{AttributeSection, Save, SaveSectionMap, map_core_sections};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: d2save_status_inspect <file.d2s>");
        return;
    }

    let bytes = fs::read(&args[1]).expect("Failed to read file");
    let save = Save::from_bytes(&bytes).expect("Failed to parse header");

    println!("=== SAVE STATUS INSPECT: {} ===", args[1]);
    println!("Header Name:  {}", save.header.char_name);
    println!("Header Level: {}", save.header.char_level);
    println!(
        "Header Class: {} ({})",
        save.header.char_class,
        d2r_core::save::class_name(save.header.char_class)
    );
    println!("File Size:    {}", save.header.file_size);

    let map = match map_core_sections(&bytes) {
        Ok(m) => m,
        Err(e) => {
            println!("Failed to map sections: {}", e);
            return;
        }
    };

    // Attributes (gf section)
    println!("\n--- Attributes (gf section at {}) ---", map.gf_pos);
    match AttributeSection::parse(&bytes, map.gf_pos, map.if_pos) {
        Ok(attrs) => {
            let is_alpha = save.header.version == 105;
            for entry in &attrs.entries {
                let name = d2r_core::data::stat_costs::STAT_COSTS
                    .iter()
                    .find(|s| s.id == entry.stat_id)
                    .map(|s| s.name.as_ref())
                    .unwrap_or("Unknown");

                println!(
                    "  StatID {:>3} {:<20}: Raw={} Actual={}",
                    entry.stat_id,
                    name,
                    entry.raw_value,
                    attrs
                        .actual_value(entry.stat_id, is_alpha)
                        .unwrap_or(entry.raw_value as i32)
                );
                if let Some(ref bits) = entry.opaque_bits {
                    println!("  [WARN] Entry has {} opaque bits", bits.len());
                }
            }
        }
        Err(e) => println!("  Failed to parse attributes: {}", e),
    }

    // Item Sections (JM markers)
    println!("\n--- Item Sections (JM markers) ---");
    for (i, &pos) in map.jm_positions.iter().enumerate() {
        let count = if pos + 4 <= bytes.len() {
            u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]])
        } else {
            0
        };
        println!("  JM[{}]: offset={} (0x{:04X}), count={}", i, pos, pos, count);
    }
}
