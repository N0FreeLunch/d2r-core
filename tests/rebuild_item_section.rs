use d2r_core::item::{HuffmanTree, Item};
use d2r_core::save::rebuild_item_section;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod common;
use common::repo_path;

fn write_rebuilt(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
}

fn read_items(path: &Path) -> io::Result<Vec<Item>> {
    let bytes = fs::read(path)?;
    let huffman = HuffmanTree::new();
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    Ok(Item::read_player_items(&bytes, &huffman, version == 105)?)
}

fn rebuild_and_write(base_path: &Path, items: &[Item], output_path: &Path) -> io::Result<Vec<u8>> {
    let base_bytes = fs::read(base_path)?;
    let huffman = HuffmanTree::new();
    let version = u32::from_le_bytes(base_bytes[4..8].try_into().unwrap_or([0; 4]));
    let rebuilt = rebuild_item_section(&base_bytes, items, &huffman, version == 105)?;
    write_rebuilt(output_path, &rebuilt)?;
    Ok(rebuilt)
}

#[test]
fn regen_empty_roundtrip() -> io::Result<()> {
    let empty_path = repo_path("tests/fixtures/savegames/original/amazon_empty.d2s");
    let items = read_items(&empty_path)?;
    let _rebuilt = rebuild_and_write(
        &empty_path,
        &items,
        Path::new("tests/fixtures/savegames/modified/generated_empty_roundtrip.d2s"),
    )?;
    let _ = _rebuilt;
    Ok(())
}

#[test]
fn regen_10_scrolls_from_empty() -> io::Result<()> {
    let empty_path = repo_path("tests/fixtures/savegames/original/amazon_empty.d2s");
    let scroll_path = repo_path("tests/fixtures/savegames/original/amazon_10_scrolls.d2s");
    let scroll_items = read_items(&scroll_path)?;
    let _rebuilt = rebuild_and_write(
        &empty_path,
        &scroll_items,
        Path::new("tests/fixtures/savegames/modified/generated_10_scrolls.d2s"),
    )?;
    let _ = _rebuilt;
    Ok(())
}

#[test]
fn regen_cleared_from_10_scrolls() -> io::Result<()> {
    let empty_path = repo_path("tests/fixtures/savegames/original/amazon_empty.d2s");
    let scroll_path = repo_path("tests/fixtures/savegames/original/amazon_10_scrolls.d2s");
    let base_items = read_items(&empty_path)?;
    let _rebuilt = rebuild_and_write(
        &scroll_path,
        &base_items,
        Path::new("tests/fixtures/savegames/modified/generated_cleared_from_10.d2s"),
    )?;
    let _ = _rebuilt;
    Ok(())
}

#[test]
fn regen_cleared_from_initial() -> io::Result<()> {
    let empty_path = repo_path("tests/fixtures/savegames/original/amazon_empty.d2s");
    let initial_path = repo_path("tests/fixtures/savegames/original/amazon_initial.d2s");
    let base_items = read_items(&empty_path)?;
    let rebuilt = rebuild_and_write(
        &initial_path,
        &base_items,
        Path::new("tests/fixtures/savegames/modified/generated_cleared_from_initial.d2s"),
    )?;
    write_rebuilt(
        Path::new("tests/fixtures/savegames/modified/generated_armor_case.d2s"),
        &rebuilt,
    )?;
    Ok(())
}
