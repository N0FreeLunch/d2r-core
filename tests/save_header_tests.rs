use std::fs;
use std::path::PathBuf;

use d2r_core::save::{find_jm_markers, recalculate_checksum, Save, D2S_MAGIC};

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_fixture(relative: &str) -> Vec<u8> {
    fs::read(repo_path(relative)).expect("fixture should be readable")
}

#[test]
fn save_header_parses_amazon_empty_header() {
    let bytes = load_fixture("tests/fixtures/savegames/original/amazon_empty.d2s");
    let save = Save::from_bytes(&bytes).expect("header parsing should succeed");

    assert_eq!(save.header.magic, D2S_MAGIC);
    assert_eq!(save.header.version, 105);
    assert_eq!(save.header.file_size as usize, bytes.len());
    assert_eq!(save.header.char_name, "TESTAMAZON");
    assert_eq!(save.header.char_class, 0);
    assert_eq!(save.header.char_level, 1);
    assert_eq!(save.header.active_weapon, 0);
}

#[test]
fn save_header_parses_all_original_amazon_headers() {
    let fixture_dir = repo_path("tests/fixtures/savegames/original");

    for entry in fs::read_dir(fixture_dir).expect("fixture directory should exist") {
        let entry = entry.expect("fixture entry should be readable");
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("d2s") {
            continue;
        }

        let bytes = fs::read(&path).expect("fixture should be readable");
        let save = Save::from_bytes(&bytes).expect("fixture header should parse");

        assert_eq!(save.header.magic, D2S_MAGIC, "{}", path.display());
        assert_eq!(save.header.version, 105, "{}", path.display());
        assert_eq!(save.header.char_class, 0, "{}", path.display());
        assert_eq!(save.header.char_level, 1, "{}", path.display());
        assert_eq!(save.header.char_name, "TESTAMAZON", "{}", path.display());
    }
}

#[test]
fn save_header_finds_expected_jm_markers_in_empty_save() {
    let bytes = load_fixture("tests/fixtures/savegames/original/amazon_empty.d2s");
    assert_eq!(find_jm_markers(&bytes), vec![903, 947]);
}

#[test]
fn save_header_recalculates_checksum_for_empty_save() {
    let bytes = load_fixture("tests/fixtures/savegames/original/amazon_empty.d2s");
    let stored = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    let recalculated = recalculate_checksum(&bytes).expect("checksum should recalculate");
    assert_eq!(stored, recalculated);
}
