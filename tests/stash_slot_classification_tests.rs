use d2r_core::item::HuffmanTree;
use d2r_core::save::{collect_player_slots, ItemSlotClass};
use std::fs;
use std::io;
use std::path::PathBuf;

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_fixture(path: &str) -> io::Result<Vec<u8>> {
    fs::read(repo_path(path))
}

#[test]
fn stash_slot_classification_progression_fixture() -> io::Result<()> {
    let bytes = load_fixture("tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s")?;
    let huffman = HuffmanTree::new();
    let slots = collect_player_slots(&bytes, &huffman)?;
    let top_level = slots
        .iter()
        .filter(|(_, class)| *class != ItemSlotClass::SocketChild)
        .count();
    assert_eq!(top_level, 16);
    assert!(slots
        .iter()
        .any(|(_, class)| *class == ItemSlotClass::StashLike));
    assert!(slots
        .iter()
        .any(|(_, class)| *class == ItemSlotClass::SocketChild));
    Ok(())
}
