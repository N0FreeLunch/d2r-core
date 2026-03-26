use d2r_core::save::{Save, map_core_sections};
use d2r_core::data::quests::V105_QUESTS;
use d2r_core::data::waypoints::WAYPOINTS;
use std::fs;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: v105_progression_dump <file.d2s>");
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).expect("Failed to read file");
    
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    if version != 105 {
        println!("Error: This tool is only for Alpha v105 (version 105). File version is {}.", version);
        return;
    }

    println!("=== Alpha v105 Progression Dump: {} ===", path);
    
    println!("\n--- Quests (Woo! section at 0x193) ---");
    for entry in V105_QUESTS {
        let offset = entry.v105_offset;
        if offset + 2 <= bytes.len() {
            let state = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            if state != 0 {
                let diff_str = match entry.difficulty {
                    0 => "Normal",
                    1 => "Nightmare",
                    2 => "Hell",
                    _ => "???",
                };
                println!("  [{:>9}] Act {} - {:<30}: Status=0x{:04X}", 
                    diff_str, entry.act, entry.name, state);
            }
        }
    }

    println!("\n--- Waypoints ---");
    // Normal WPs: 0x19B (2 bytes per act)
    println!("  [Normal] (at 0x19B)");
    let normal_wp_start = 0x19B;
    for act in 1..=5 {
        let act_offset = normal_wp_start + (act - 1) * 2;
        if act_offset + 2 <= bytes.len() {
            let val = u16::from_le_bytes([bytes[act_offset], bytes[act_offset+1]]);
            for wp in WAYPOINTS.iter().filter(|w| w.act == act as u8) {
                if (val & (1 << wp.index)) != 0 {
                    println!("    Act {} - {:<20}: ACTIVE (0x{:04X})", act, wp.name, val);
                }
            }
        }
    }

    // NM WPs: 0x2C7
    println!("  [Nightmare] (at 0x2C7)");
    let nm_wp_start = 0x2C7;
    for act in 1..=5 {
        let act_offset = nm_wp_start + (act - 1) * 2;
        if act_offset + 2 <= bytes.len() {
            let val = u16::from_le_bytes([bytes[act_offset], bytes[act_offset+1]]);
            for wp in WAYPOINTS.iter().filter(|w| w.act == act as u8) {
                if (val & (1 << wp.index)) != 0 {
                    println!("    Act {} - {:<20}: ACTIVE (0x{:04X})", act, wp.name, val);
                }
            }
        }
    }

    // Hell WPs: 0x2CB
    println!("  [Hell] (at 0x2CB)");
    let hell_wp_start = 0x2CB;
    for act in 1..=5 {
        let act_offset = hell_wp_start + (act - 1) * 2;
        if act_offset + 2 <= bytes.len() {
            let val = u16::from_le_bytes([bytes[act_offset], bytes[act_offset+1]]);
            for wp in WAYPOINTS.iter().filter(|w| w.act == act as u8) {
                if (val & (1 << wp.index)) != 0 {
                    println!("    Act {} - {:<20}: ACTIVE (0x{:04X})", act, wp.name, val);
                }
            }
        }
    }
}
