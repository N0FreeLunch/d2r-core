use crate::data::stat_costs::STAT_COSTS;
pub mod entity;
pub mod axiom;
pub mod parser;

pub use entity::{ItemProperty, ItemStats, AlphaStatMap, ALPHA_STAT_MAPS};
pub use axiom::StatsAxiom;
pub use parser::{read_property_list, parse_single_property, PropertyParseResult};

pub fn lookup_alpha_map_by_raw(raw_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.raw_id == raw_id)
}

pub fn lookup_alpha_map_by_effective(effective_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.effective_id == effective_id)
}

pub fn stat_save_bits(stat_id: u32) -> Option<u32> {
    STAT_COSTS
        .iter()
        .find(|stat| stat.id == stat_id)
        .map(|stat| stat.save_bits as u32)
}
