pub mod entity;
pub mod axiom;

pub use entity::{ItemProperty, ItemStats, AlphaStatMap, ALPHA_STAT_MAPS};
pub use axiom::StatsAxiom;

pub fn lookup_alpha_map_by_raw(raw_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.raw_id == raw_id)
}

pub fn lookup_alpha_map_by_effective(effective_id: u32) -> Option<&'static AlphaStatMap> {
    ALPHA_STAT_MAPS.iter().find(|m| m.effective_id == effective_id)
}
