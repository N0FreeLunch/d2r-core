pub mod spec;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_load_dlc_spec() {
        let yaml_str = fs::read_to_string("../d2r-spec/specification/v2_dlc_spec.yaml")
            .expect("Should have been able to read the file");
        let spec: spec::DlcSpec = serde_yaml::from_str(&yaml_str).expect("Failed to parse YAML");

        assert_eq!(spec.name, "Reign of the Demonologist");
        assert_eq!(spec.character_classes[0].name, "Warlock");
        assert_eq!(spec.character_classes[0].id, 7);
        assert_eq!(spec.character_classes[0].skills.len(), 30);
    }
}
