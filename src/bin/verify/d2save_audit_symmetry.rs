use d2r_core::data::quests::V105_QUESTS;
use d2r_core::data::waypoints::WAYPOINTS;
use d2r_core::domain::progression::quest::V105_QUEST_NORMAL_START_FILE;
use std::process;

fn main() {
    println!("--- Alpha v105 Grounding Symmetry Audit ---");
    let mut failures = 0;

    // 1. Quest Audit
    println!("Auditing Alpha v105 Quests...");
    for (i, entry) in V105_QUESTS.iter().enumerate() {
        if entry.v105_offset < V105_QUEST_NORMAL_START_FILE {
            println!("  [FAIL] Quest[{}]: {} offset {} is below grounding anchor {}", i, entry.name, entry.v105_offset, V105_QUEST_NORMAL_START_FILE);
            failures += 1;
        } else {
            let relative = entry.v105_offset - V105_QUEST_NORMAL_START_FILE;
            if relative % 4 != 0 {
                // Actually in Alpha v105, they are spaced by 4 bytes (stride 4)
                println!("  [FAIL] Quest[{}]: {} offset {} is not 4-byte aligned relative to anchor {}", i, entry.name, entry.v105_offset, V105_QUEST_NORMAL_START_FILE);
                failures += 1;
            }
        }
    }
    if failures == 0 {
        println!("  [OK] Quest grounding looks consistent.");
    }

    // 2. Waypoint Audit
    println!("Auditing Alpha v105 Waypoints...");
    // Alpha v105 has 24 bytes per difficulty for waypoints (192 bits).
    let max_ws_bit = 24 * 8; 
    for (i, entry) in WAYPOINTS.iter().enumerate() {
        if entry.ws_bit as usize >= max_ws_bit {
            println!("  [FAIL] Waypoint[{}]: {} ws_bit {} exceeds difficulty stride {}", i, entry.name, entry.ws_bit, max_ws_bit);
            failures += 1;
        }
    }
    if failures == 0 {
        println!("  [OK] Waypoint bit projections look consistent.");
    }

    if failures > 0 {
        println!("\nAudit FAILED with {} error(s).", failures);
        process::exit(1);
    } else {
        println!("\nAudit PASSED.");
    }
}