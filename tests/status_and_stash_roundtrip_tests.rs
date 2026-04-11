use d2r_core::item::{HuffmanTree, Item};
use d2r_core::save::{
    AttributeSection, map_core_sections, parse_quest_section, parse_skill_section,
    rebuild_status_and_player_items,
};
use std::fs;
use std::io;

mod common;
use common::repo_path;

fn load_fixture(path: &str) -> io::Result<Vec<u8>> {
    fs::read(repo_path(path))
}

#[test]
fn status_and_stash_roundtrip_fixtures() -> io::Result<()> {
    let fixtures = [
        "tests/fixtures/savegames/original/amazon_empty.d2s",
        "tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s",
    ];
    let huffman = HuffmanTree::new();
    for fixture in fixtures {
        let bytes = load_fixture(fixture)?;
        let map = map_core_sections(&bytes)?;
        let attributes = AttributeSection::parse(&bytes, map.gf_pos, map.if_pos)?;
        let skills = parse_skill_section(&bytes, &map)?;
        let quests = parse_quest_section(&bytes, &map)?;
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
        let items = Item::read_player_items(&bytes, &huffman, version == 105)?;
        let rebuilt = rebuild_status_and_player_items(
            &bytes,
            Some(&attributes),
            Some(&skills),
            Some(&quests),
            None,
            None,
            &items,
            &huffman,
        )?;
        assert_eq!(rebuilt, bytes);
    }
    Ok(())
}

#[test]
fn test_level_and_header_sync() -> io::Result<()> {
    let bytes = load_fixture("tests/fixtures/savegames/original/amazon_empty.d2s")?;
    let huffman = HuffmanTree::new();

    // Level 1 -> 99
    let patched = d2r_core::save::patch_level(&bytes, 99, &huffman)?;

    let map = map_core_sections(&patched)?;
    let attributes = AttributeSection::parse(&patched, map.gf_pos, map.if_pos)?;

    assert_eq!(
        patched[d2r_core::save::CHAR_LEVEL_OFFSET],
        99,
        "Header level should be 99"
    );
    assert_eq!(
        attributes.actual_value(12, true),
        Some(99),
        "GF level should be 99"
    );

    // Skill patch test
    let mut skills = d2r_core::save::parse_skill_section(&patched, &map)?;
    // Amazon Critical Strike is index 3 (ID 9)
    // We update it to level 5
    let mut data = *skills.as_slice();
    data[3] = 5;
    let skills_updated = d2r_core::save::SkillSection::from_slice(&data)?;

    let version = u32::from_le_bytes(patched[4..8].try_into().unwrap_or([0; 4]));
    let items = Item::read_player_items(&patched, &huffman, version == 105)?;
    let final_rebuilt = rebuild_status_and_player_items(
        &patched,
        Some(&attributes),
        Some(&skills_updated),
        None,
        None,
        None,
        &items,
        &huffman,
    )?;

    let final_map = map_core_sections(&final_rebuilt)?;
    let final_skills = d2r_core::save::parse_skill_section(&final_rebuilt, &final_map)?;
    assert_eq!(final_skills.as_slice()[3], 5, "Critical Strike should be 5");
    Ok(())
}

#[test]
fn test_variable_length_rebuild() -> io::Result<()> {
    let bytes = fs::read(repo_path(
        "tests/fixtures/savegames/original/amazon_empty.d2s",
    ))?;
    let huffman = HuffmanTree::new();
    let map = map_core_sections(&bytes)?;
    let mut attrs = AttributeSection::parse(&bytes, map.gf_pos, map.if_pos)?;

    let original_len = bytes.len();

    // Add a stat ID 16 (item_armor_percent) which has 9 bits in stat_costs.rs
    // This is not in the special character stats list, so it tests the fallback/dynamic path.
    attrs.set_raw(16, 42);

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    let items = Item::read_player_items(&bytes, &huffman, version == 105)?;
    let rebuilt = rebuild_status_and_player_items(
        &bytes,
        Some(&attrs),
        None,
        None,
        None,
        None,
        &items,
        &huffman,
    )?;

    assert!(
        rebuilt.len() >= original_len,
        "Rebuilt save should be at least as large as original"
    );

    // Check that we can parse it back
    let new_map = map_core_sections(&rebuilt)?;
    let new_attrs = AttributeSection::parse(&rebuilt, new_map.gf_pos, new_map.if_pos)?;

    let found = new_attrs
        .entries
        .iter()
        .any(|e| e.stat_id == 16 && e.raw_value == 42);
    assert!(found, "New attribute entry (ID 16) should be preserved");

    let file_size_in_header = u32::from_le_bytes(rebuilt[8..12].try_into().unwrap());
    assert_eq!(
        file_size_in_header,
        rebuilt.len() as u32,
        "Header file size should be updated"
    );

    Ok(())
}
