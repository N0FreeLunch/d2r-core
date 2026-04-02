use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct V105BitConfig {
    pub location_bits: u32,
    pub universal_gap_bits: u32,
}

/// Returns the Alpha v105 bit-width axioms from the contract.
pub fn v105_bits() -> V105BitConfig {
    let json = include_str!("../../../d2r-data/constants/v105_bits.json");
    serde_json::from_str(json).expect("Failed to parse v105_bits.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_v105_axioms_load() {
        let axioms = v105_bits();
        assert_eq!(axioms.location_bits, 3);
        assert_eq!(axioms.universal_gap_bits, 8);
    }
}
