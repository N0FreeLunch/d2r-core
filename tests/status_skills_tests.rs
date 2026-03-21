use d2r_core::save::{map_core_sections, parse_skill_section, patch_skill_section};
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
fn status_skills_parse_patch_roundtrip() -> io::Result<()> {
    let empty_bytes = load_fixture("tests/fixtures/savegames/original/amazon_empty.d2s")?;
    let map_empty = map_core_sections(&empty_bytes)?;
    let skills_empty = parse_skill_section(&empty_bytes, &map_empty)?;
    assert!(skills_empty.as_slice().iter().all(|&b| b == 0));
    let rebuilt_empty = patch_skill_section(&empty_bytes, &map_empty, &skills_empty)?;
    assert_eq!(rebuilt_empty, empty_bytes);

    let prog_bytes =
        load_fixture("tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s")?;
    let map_prog = map_core_sections(&prog_bytes)?;
    let skills_prog = parse_skill_section(&prog_bytes, &map_prog)?;
    assert_eq!(skills_prog.as_slice()[3], 1);
    let patched_prog = patch_skill_section(&prog_bytes, &map_prog, &skills_prog)?;
    assert_eq!(patched_prog, prog_bytes);
    Ok(())
}
