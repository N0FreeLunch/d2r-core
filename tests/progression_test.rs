use d2r_core::item::HuffmanTree;
use d2r_core::save::{Save, rebuild_status_and_player_items, WaypointSection, QuestSection, ExpansionSection};
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

    let mut wps = save.header.waypoints.clone().expect("v105 should have waypoints");
    let mut quests = save.header.quests.clone().expect("v105 should have quests");
    let mut expansion = save.header.expansion.clone().expect("v105 should have expansion");

    // Check Initial Waypoint (e.g. byte 10 is 0)
    assert_eq!(wps.raw_bytes[10], 0x00);

    // 2. Perform Mutations
    wps.raw_bytes[10] = 0x55;
    quests.raw_bytes[0] = 0xAA;
    expansion.raw_bytes[5] = 0xFF;

    // 3. Rebuild Save
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    let items = d2r_core::item::Item::read_player_items(&bytes, &huffman, version == 105)?;
    let rebuilt = rebuild_status_and_player_items(
        &bytes,
        None, // attributes
        None, // skills
        Some(&quests),
        Some(&wps),
        Some(&expansion),
        &items,
        &huffman,
    )?;

    // 4. Verify rebuilt save matches expectations
    // Check offsets (v105 fixed offsets)
    // Quests: 0x19F
    assert_eq!(rebuilt[0x19F], 0xAA);
    // Waypoints: 0x193 + 10 = 0x19D
    assert_eq!(rebuilt[0x19D], 0x55);
    // Expansion: 0x2BD + 5 = 0x2C2
    assert_eq!(rebuilt[0x2C2], 0xFF);

    // 5. Verify roundtrip parsing
    let save_back = Save::from_bytes(&rebuilt)?;
    assert_eq!(save_back.header.version, 105);
    assert_eq!(save_back.header.quests.unwrap().raw_bytes[0], 0xAA);
    assert_eq!(save_back.header.waypoints.unwrap().raw_bytes[10], 0x55);
    assert_eq!(save_back.header.expansion.unwrap().raw_bytes[5], 0xFF);

    Ok(())
}

#[test]
fn test_alpha_v105_waypoint_name_mapping() -> std::io::Result<()> {
    let mut wps = WaypointSection::from_slice(&[0u8; 80]);
    let mut ex = ExpansionSection::from_slice(&[0u8; 80]);

    // Act 1 Town (Index 0 in Act 1 Block)
    wps.set_activated_by_name("Act 1 - Town", true);
    ex.set_activated_by_name("Act 1 - Town", 1, true);

    // Expected in WaypointSection (Woo!): byte 8 (offset 0), bit 0
    assert_eq!(wps.raw_bytes[8], 0x01);
    // Expected in ExpansionSection (WS): byte 10 (offset 0), bit 0
    assert_eq!(ex.raw_bytes[10], 0x01);

    // Act 2 Town (Index 0 in Act 2 Block, so bit 16 overall)
    wps.set_activated_by_name("Act 2 - Town", true);
    ex.set_activated_by_name("Act 2 - Town", 1, true);

    // Act 2 starts at 2 bytes (16 bits) after Act 1
    // In WaypointSection: byte 10 (offset 2), bit 0
    // (Wait: 8*8 + 16 = 80 bits = byte 10, bit 0)
    assert_eq!(wps.raw_bytes[10], 0x01);
    // In ExpansionSection: byte 12 (offset 2), bit 0
    assert_eq!(ex.raw_bytes[12], 0x01);

    Ok(())
}
