use d2r_core::data::quests::V105_QUESTS;
use d2r_core::data::waypoints::WAYPOINTS;
use d2r_core::save::Save;
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run --example v105_unlocker -- <input.d2s> <output.d2s>");
        process::exit(1);
    }

    let input_path = &args[1];
    let output_path = &args[2];

    println!("Loading save from: {}", input_path);
    let mut bytes = fs::read(input_path).expect("Failed to read input file");
    
    let mut save = Save::from_bytes(&bytes).expect("Failed to parse save file");
    
    if save.header.version != 105 {
        println!("Error: This tool only supports Alpha v105 (version 105) save files.");
        process::exit(1);
    }

    // 1. Quests
    println!("Unlocking all quests for all difficulties...");
    if let Some(ref mut quests) = save.header.quests {
        for quest in V105_QUESTS.iter() {
            quests.set_v105_completed_by_name(quest.name, true);
        }
    } else {
        println!("  (No Quest section found in header)");
    }

    // 2. Waypoints (Normal)
    println!("Unlocking all Normal waypoints...");
    if let Some(ref mut wp) = save.header.waypoints {
        for entry in WAYPOINTS.iter() {
            wp.set_activated_by_name(entry.name, true);
        }
    } else {
        println!("  (No Normal Waypoint section found in header)");
    }

    // 3. Waypoints (Nightmare/Hell)
    println!("Unlocking all NM/Hell waypoints...");
    if let Some(ref mut expansion) = save.header.expansion {
        for diff in 1..=2 {
            for entry in WAYPOINTS.iter() {
                expansion.set_activated_by_name(entry.name, diff, true);
            }
        }
    } else {
        println!("  (No Expansion section found in header)");
    }

    println!("Writing unlocked save to: {}", output_path);
    save.apply_header_to_bytes(&mut bytes).expect("Failed to apply header changes");
    fs::write(output_path, bytes).expect("Failed to write output file");

    println!("Done!");
    println!("Verify results: cargo run --bin d2item_chunk_verify -- {}", output_path);
}
