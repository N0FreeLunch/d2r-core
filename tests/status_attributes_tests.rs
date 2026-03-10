use d2r_core::save::{gf_payload_range, map_core_sections, AttributeSection};
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
fn status_attributes_progression_values_roundtrip() -> io::Result<()> {
    let bytes =
        load_fixture("tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s")?;
    let map = map_core_sections(&bytes)?;
    let section = AttributeSection::parse(&bytes, &map)?;
    assert_eq!(section.entries.len(), 26);
    assert_eq!(section.actual_value(0), Some(-12));
    assert_eq!(section.actual_value(4), Some(0));

    let range = gf_payload_range(&map);
    let serialized = section.to_bytes()?;
    let range_clone = range.clone();
    let slice = &bytes[range_clone];
    assert_eq!(serialized.len(), slice.len());
    assert_eq!(&serialized[..], slice);
    Ok(())
}
