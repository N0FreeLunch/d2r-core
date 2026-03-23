use d2r_core::item::HuffmanTree;
use d2r_core::save::{Save, rebuild_status_and_player_items};
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
fn test_waypoint_activation_roundtrip() -> std::io::Result<()> {
    let path = repo_path("tests/fixtures/savegames/original/amazon_initial.d2s");
    let bytes = fs::read(path).expect("fixture should be readable");
    let huffman = HuffmanTree::new();

    // 1. Initial State Check
    let save = Save::from_bytes(&bytes)?;
    let mut wps = save.header.waypoints.expect("initial save should have waypoints for v105");
    
    // Index 10 is 0x19D. Initial is 00.
    assert_eq!(wps.raw_bytes[10], 0x00, "Initial waypoint byte should be 00");

    // 2. Activate Waypoint
    wps.set_activated(10, 0, true);
    assert_eq!(wps.raw_bytes[10], 0x01, "Waypoint byte should be 01 after activation");

    // 3. Rebuild Save
    // No changes to items/attributes/skills
    let rebuilt = rebuild_status_and_player_items(
        &bytes,
        None,
        None,
        None,
        Some(&wps),
        None,
        &d2r_core::item::Item::read_player_items(&bytes, &huffman)?,
        &huffman,
    )?;

    // 4. Verify binary change
    assert_eq!(rebuilt[0x19D], 0x01, "Rebuilt save should have 01 at 0x19D");
    
    // Verify that everything else is the same (except checksum)
    assert_eq!(rebuilt.len(), bytes.len());
    
    // Verify it still parses back
    let save_back = Save::from_bytes(&rebuilt)?;
    assert_eq!(save_back.header.waypoints.unwrap().raw_bytes[10], 0x01);

    Ok(())
}
