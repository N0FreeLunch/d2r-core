use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::data::stat_costs::STAT_COSTS;
use d2r_core::save::Save;
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

    // Find gf marker (Attributes)
    if let Some(gf_pos) = (0..bytes.len() - 2).find(|&i| bytes[i] == b'g' && bytes[i + 1] == b'f') {
        println!("\n--- Attributes (gf section at {}) ---", gf_pos);
        let mut reader = BitReader::endian(Cursor::new(&bytes[gf_pos + 2..]), LittleEndian);
        loop {
            let stat_id = reader.read::<9, u32>().unwrap_or(0x1FF);
            if stat_id == 0x1FF {
                println!("Terminator 0x1FF found");
                break;
            }

            let cost = STAT_COSTS.iter().find(|s| s.id == stat_id);
            match cost {
                Some(c) => {
                    let mut val = 0i32;
                    if c.save_bits > 0 {
                        val = reader.read_var::<u32>(c.save_bits as u32).unwrap_or(0) as i32;
                    }
                    println!(
                        "  StatID {:>3} {:<20}: Raw={} Actual={}",
                        stat_id,
                        c.name,
                        val,
                        val - c.save_add
                    );
                }
                None => {
                    println!("  Unknown StatID {} - Parsing broken", stat_id);
                    break;
                }
            }
        }
    }

    // Find if marker (Skills)
    if let Some(if_pos) = (0..bytes.len() - 2).find(|&i| bytes[i] == b'i' && bytes[i + 1] == b'f') {
        println!("\n--- Skills (if section at {}) ---", if_pos);
        if if_pos + 2 + 30 > bytes.len() {
            println!("Skill section is truncated (expected 30 bytes)");
        } else {
            // Fixed size 30 bytes for skill points per class
            let skill_bytes = &bytes[if_pos + 2..if_pos + 2 + 30];
            for (i, &lvl) in skill_bytes.iter().enumerate() {
                if lvl > 0 {
                    println!("  Skill Index {:>2}: Level {}", i, lvl);
                }
            }
        }
    }
}
