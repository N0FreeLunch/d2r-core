use d2r_core::item::{HuffmanTree, Item};
use d2r_core::save::{
    AttributeSection, map_core_sections, parse_skill_section, rebuild_status_and_player_items,
};
use std::fs;
use std::io;
use std::path::PathBuf;

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
        let attributes = AttributeSection::parse(&bytes, &map)?;
        let skills = parse_skill_section(&bytes, &map)?;
        let items = Item::read_player_items(&bytes, &huffman)?;
        let rebuilt = rebuild_status_and_player_items(
            &bytes,
            Some(&attributes),
            Some(&skills),
            &items,
            &huffman,
        )?;
        assert_eq!(rebuilt, bytes);
    }
    Ok(())
}
