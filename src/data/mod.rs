#[path = "../../d2r-data/mod.rs"]
pub mod generated;

pub use generated::{
    affixes, item_codes, item_specs, item_types, localization, monsters, property_map,
    runewords, set_items, skills, stat_costs, unique_items,
    legitimacy,
};
