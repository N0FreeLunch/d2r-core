use crate::domain::item::quality::ItemQuality;
use super::entity::{ALPHA_STAT_MAPS, AlphaStatMap};
use crate::data::stat_costs::STAT_COSTS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatsAxiom {
    pub version: u8,
    pub quality: ItemQuality,
}

impl StatsAxiom {
    pub fn new(version: u8, quality: ItemQuality) -> Self {
        Self { version, quality }
    }

    pub fn is_alpha(&self) -> bool {
        self.version == 5 || self.version == 1
    }

    /// Maps an Alpha v105 raw stat ID to its effective (standard) ID.
    pub fn map_alpha_id(&self, raw_id: u32) -> u32 {
        if !self.is_alpha() {
            return raw_id;
        }
        ALPHA_STAT_MAPS
            .iter()
            .find(|m| m.raw_id == raw_id)
            .map(|m| m.effective_id)
            .unwrap_or(raw_id)
    }

    /// Determines the bit width of a property value based on the stat ID and item quality.
    pub fn property_bit_width(&self, stat_id: u32) -> u32 {
        if self.is_alpha() {
            // Alpha v105 Quality-dependent property widths:
            // Normal items use 0 bits for value (ID only).
            // Others (Magic/Rare/Unique/Set) use 1 bit as part of a 10-bit property model.
            if self.quality == ItemQuality::Normal {
                0
            } else {
                1
            }
        } else {
            // Standard behavior: lookup in STAT_COSTS
            super::stat_save_bits(stat_id).unwrap_or(0)
        }
    }

    pub fn lookup_alpha_map_by_raw(&self, raw_id: u32) -> Option<&'static AlphaStatMap> {
        ALPHA_STAT_MAPS.iter().find(|m| m.raw_id == raw_id)
    }

    pub fn lookup_alpha_map_by_effective(&self, effective_id: u32) -> Option<&'static AlphaStatMap> {
        ALPHA_STAT_MAPS.iter().find(|m| m.effective_id == effective_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alpha_id_mapping() {
        let axiom = StatsAxiom::new(5, ItemQuality::Unique);
        assert_eq!(axiom.map_alpha_id(256), 127); // item_allskills
        assert_eq!(axiom.map_alpha_id(496), 99);  // item_fastergethitrate
        assert_eq!(axiom.map_alpha_id(999), 999); // identity mapping for unknown
    }

    #[test]
    fn test_alpha_bit_widths() {
        let normal_axiom = StatsAxiom::new(5, ItemQuality::Normal);
        assert_eq!(normal_axiom.property_bit_width(256), 0);

        let magic_axiom = StatsAxiom::new(5, ItemQuality::Magic);
        assert_eq!(magic_axiom.property_bit_width(256), 1); // 1-bit value + 9-bit ID = 10-bit model
    }
}
