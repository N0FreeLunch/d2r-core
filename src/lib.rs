pub mod data;
pub mod inventory;
pub mod item;
pub mod save;
pub mod spec;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::HuffmanTree;
    use crate::item::Item;
    use std::fs;
    use std::path::PathBuf;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn load_player_items(relative: &str) -> Vec<Item> {
        let bytes = fs::read(repo_path(relative)).expect("fixture should be readable");
        let huffman = HuffmanTree::new();
        Item::read_player_items(&bytes, &huffman).expect("item parse should succeed")
    }

    #[test]
    fn test_load_dlc_spec() {
        let yaml_str = fs::read_to_string(repo_path("../d2r-spec/specification/v2_dlc_spec.yaml"))
            .expect("Should have been able to read the file");
        let spec: spec::DlcSpec = serde_yaml::from_str(&yaml_str).expect("Failed to parse YAML");

        assert_eq!(spec.name, "Reign of the Demonologist");
        assert_eq!(spec.character_classes[0].name, "Warlock");
        assert_eq!(spec.character_classes[0].id, 7);
        assert_eq!(spec.character_classes[0].skills.len(), 30);
    }

    #[test]
    fn test_runeword_socket_children_are_recovered() {
        let items =
            load_player_items("tests/fixtures/savegames/original/amazon_authority_runeword.d2s");

        assert_eq!(items.len(), 6);

        let authority = items
            .iter()
            .find(|item| item.code.trim() == "w ha")
            .expect("authority base item should be present");

        let child_codes: Vec<&str> = authority
            .socketed_items
            .iter()
            .map(|item| item.code.trim())
            .collect();

        assert_eq!(child_codes, vec!["r15", "r13", "r08"]);
    }

    #[test]
    fn test_plain_inventory_fixture_does_not_gain_socket_children() {
        let items = load_player_items("tests/fixtures/savegames/original/amazon_10_scrolls.d2s");

        assert_eq!(items.len(), 16);
        assert!(items.iter().all(|item| item.socketed_items.is_empty()));
    }

    #[test]
    fn test_authority_runeword_children_stay_nested_with_expected_modes() {
        let items =
            load_player_items("tests/fixtures/savegames/original/amazon_authority_runeword.d2s");

        let top_level_codes: Vec<&str> = items.iter().map(|item| item.code.trim()).collect();
        assert_eq!(
            top_level_codes,
            vec!["hp1", "hp1", "hp1", "hp1", "xrs", "w ha"]
        );

        let authority = items.last().expect("authority base item should be last");
        assert_eq!(authority.code.trim(), "w ha");

        let child_summaries: Vec<(&str, u8)> = authority
            .socketed_items
            .iter()
            .map(|item| (item.code.trim(), item.mode))
            .collect();
        assert_eq!(child_summaries, vec![("r15", 6), ("r13", 6), ("r08", 6)]);

        assert!(
            items
                .iter()
                .all(|item| !matches!(item.code.trim(), "r15" | "r13" | "r08"))
        );
    }
}
