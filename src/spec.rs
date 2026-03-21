use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DlcSpec {
    pub version: String,
    pub name: String,
    pub release_year: u32,
    pub character_classes: Vec<CharacterClass>,
    pub item_properties: Vec<ItemProperty>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CharacterClass {
    pub id: u32,
    pub code: String,
    pub name: String,
    pub is_dlc: bool,
    pub starting_stats: StartingStats,
    pub skills: Vec<Skill>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StartingStats {
    pub str: u32,
    pub dex: u32,
    pub vit: u32,
    pub ene: u32,
    pub hp: u32,
    pub mana: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Skill {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemProperty {
    pub id: u32,
    pub code: String,
    pub description: String,
}
