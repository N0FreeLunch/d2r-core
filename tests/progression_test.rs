use d2r_core::domain::progression::quest::QuestSet;
use d2r_core::domain::progression::waypoint::WaypointSet;
use d2r_core::item::HuffmanTree;
use d2r_core::save::{
    ExpansionSection, QuestSection, Save, WaypointSection, rebuild_status_and_player_items,
};
use std::fs;
use std::path::PathBuf;

fn repo_path(relative: &str) -> PathBuf {
    let _ = dotenvy::dotenv();
    let base = std::env::var("D2R_CORE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    base.join(relative)
}

#[test]
fn test_alpha_v105_progression_mutation_verification() -> std::io::Result<()> {
    let path = repo_path("tests/fixtures/savegames/original/amazon_empty.d2s");
    let bytes = fs::read(path).expect("fixture should be readable");
    let huffman = HuffmanTree::new();

    // 1. Initial State Check
    let save = Save::from_bytes(&bytes)?;
    assert_eq!(save.header.version, 105);

    let mut wps_section = save
        .header
        .waypoints
        .clone()
        .expect("v105 should have waypoints");
    let mut quests_section = save.header.quests.clone().expect("v105 should have quests");
    let mut expansion = save
        .header
        .expansion
        .clone()
        .expect("v105 should have expansion");

    // Capture "before" state for no-collateral verification
    let wps_before = wps_section.raw_bytes.clone();
    let quests_before = quests_section.raw_bytes.clone();

    // 2. Perform Mutations via Domain Models
    
    // 2a. Waypoint Mutation
    let mut wp_set = WaypointSet::from_bytes(&wps_section.raw_bytes, 0); // Normal
    {
        let name = "Act 1 - Wilderness 2";
        let mut wp = wp_set.find_by_name(name).expect("Should find Cold Plains (Wilderness 2)");
        assert!(!wp.is_active());
        wp.set_active(true);
        // Note: find_by_name returns a copy, so we need to update the set if it's not a reference
        // Actually, WaypointSet::waypoints_mut gives access
        let wp_mut = wp_set.waypoints_mut().iter_mut().find(|w| w.name() == name).unwrap();
        wp_mut.set_active(true);
    }
    wp_set.sync_to_bytes(&mut wps_section.raw_bytes);

    // 2b. Quest Mutation
    let mut q_set = QuestSet::from_v105_bytes(&quests_section.raw_bytes);
    {
        let den = q_set.find_by_name("Den of Evil").expect("Should find Den of Evil");
        assert!(!den.is_completed());
        let q_mut = q_set.quests_mut().iter_mut().find(|q| q.name() == "Den of Evil").unwrap();
        q_mut.set_completed(true);
    }
    q_set.sync_to_v105_bytes(&mut quests_section.raw_bytes);

    // 2c. Expansion (kept as raw for now as no domain model exists)
    expansion.raw_bytes[5] = 0xFF;

    // 3. No-Collateral Verification (Inside Sections)
    
    // Waypoint: "Act 1 - Wilderness 2" is ws_bit 1 (Act 1). 
    // Normal starts at bit 80 (byte 10). bit 81 is byte 10, bit 1.
    // 0x01 (Act 1 Town) | 0x02 (Cold Plains) = 0x03
    assert_eq!(wps_section.raw_bytes[10], 0x03);
    for i in 0..wps_section.raw_bytes.len() {
        if i == 10 { continue; }
        assert_eq!(wps_section.raw_bytes[i], wps_before[i], "Waypoint byte {} should not change", i);
    }

    // Quest: "Den of Evil" is at offset 12 in raw_bytes (415 - 403 = 12)
    // set_completed(true) sets bit 0 (byte 12) and bit 12 (0x10 in byte 13)
    assert_eq!(quests_section.raw_bytes[12], 0x01);
    assert_eq!(quests_section.raw_bytes[13], 0x10);
    
    // Quest Section Marker: "Woo!" (0x193 = 403)
    assert_eq!(quests_section.raw_bytes[0], 0x57); // 'W'
    for i in 0..quests_section.raw_bytes.len() {
        if i == 12 || i == 13 { continue; }
        assert_eq!(quests_section.raw_bytes[i], quests_before[i], "Quest byte {} should not change", i);
    }

    // 4. Rebuild Save
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    let items = d2r_core::item::Item::read_player_items(&bytes, &huffman, version == 105)?;
    let rebuilt = rebuild_status_and_player_items(
        &bytes,
        None, // attributes
        None, // skills
        Some(&quests_section),
        Some(&wps_section),
        Some(&expansion),
        &items,
        &huffman,
    )?;

    // 5. Verify rebuilt save matches expectations
    // Quests: Den of Evil is at 415 (0x19F)
    assert_eq!(rebuilt[0x19F], 0x01);
    assert_eq!(rebuilt[0x1A0], 0x10);
    // Waypoints: 0x2BD + 10 = 0x2C7
    assert_eq!(rebuilt[0x2C7], 0x03);

    // 6. Verify roundtrip parsing
    let save_back = Save::from_bytes(&rebuilt)?;
    assert_eq!(save_back.header.version, 105);
    assert_eq!(save_back.header.quests.as_ref().unwrap().raw_bytes[12], 0x01);
    assert_eq!(save_back.header.quests.as_ref().unwrap().raw_bytes[13], 0x10);
    assert_eq!(save_back.header.waypoints.as_ref().unwrap().raw_bytes[10], 0x03);

    Ok(())
}

#[test]
fn test_alpha_v105_waypoint_name_mapping() -> std::io::Result<()> {
    let mut wps = WaypointSection::from_slice(&[0u8; 80]);
    let mut ex = ExpansionSection::from_slice(&[0u8; 80]);

    // Act 1 Town (Index 0 in Act 1 Block, ws_bit 0)
    wps.set_activated_by_name("Act 1 - Town", 0, true);
    ex.set_activated_by_name("Act 1 - Town", 1, true);

    assert_eq!(wps.raw_bytes[10], 0x01);
    assert_eq!(ex.raw_bytes[34], 0x01);

    // Act 2 Town (Index 0 in Act 2 Block, ws_bit 9)
    wps.set_activated_by_name("Act 2 - Town", 0, true);
    ex.set_activated_by_name("Act 2 - Town", 1, true);

    assert_eq!(wps.raw_bytes[11], 0x02);
    assert_eq!(ex.raw_bytes[35], 0x02);

    Ok(())
}
