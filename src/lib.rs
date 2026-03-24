// Copyright 2026 N0FreeLunch (https://github.com/N0FreeLunch/d2r-core)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

pub mod data;
pub mod domain;
pub mod engine;
pub mod error;
pub mod inventory;
pub mod item;
pub mod save;
pub mod spec;
pub mod algo;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::HuffmanTree;
    use crate::item::Item;
    use std::fs;
    use std::path::PathBuf;

    fn repo_path(relative: &str) -> PathBuf {
        let _ = dotenvy::dotenv();
        let base = std::env::var("D2R_CORE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));
        base.join(relative)
    }

    fn load_player_items(relative: &str) -> Vec<Item> {
        let bytes = fs::read(repo_path(relative)).expect("fixture should be readable");
        let huffman = HuffmanTree::new();
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
        Item::read_player_items(&bytes, &huffman, version == 105).expect("item parse should succeed")
    }

    #[test]
    fn test_load_dlc_spec() {
        let _ = dotenvy::dotenv();
        let spec_path = std::env::var("D2R_SPEC_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| repo_path("../d2r-spec"));
        
        let yaml_path = spec_path.join("specification/v2_dlc_spec.yaml");
        let yaml_str = fs::read_to_string(yaml_path).expect("Should have been able to read the file");
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
            .find(|item| item.code.trim() == "xrs" && item.is_runeword)
            .expect("authority base item (xrs) should be present");

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
            vec!["hp1", "hp1", "hp1", "hp1", "xrs", "xrs"]
        );

        let authority = items.last().expect("authority base item should be last");
        assert_eq!(authority.code.trim(), "xrs");

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
    #[test]
    fn test_calc_alvl() {
        use crate::data::legitimacy::calc_alvl;
        // ilvl=50, qlvl=30, magic_lvl=0 -> temp=50, 50 < 99-30/2=84, alvl=50-15=35
        assert_eq!(calc_alvl(50, 30, 0), 35);
        // High level case: ilvl=99, qlvl=30, magic_lvl=0 -> temp=99, 99 >= 84, alvl=2*99-99=99
        assert_eq!(calc_alvl(99, 30, 0), 99);
        // Magic level case: ilvl=50, qlvl=30, magic_lvl=10 -> temp=50, alvl=50+10=60
        assert_eq!(calc_alvl(50, 30, 10), 60);
    }
}
