#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ItemQuality {
    Low = 1,
    Normal = 2,
    High = 3,
    Magic = 4,
    Set = 5,
    Rare = 6,
    Unique = 7,
    Crafted = 8,
}

impl From<u8> for ItemQuality {
    fn from(v: u8) -> Self {
        match v {
            1 => ItemQuality::Low,
            3 => ItemQuality::High,
            4 => ItemQuality::Magic,
            5 => ItemQuality::Set,
            6 => ItemQuality::Rare,
            7 => ItemQuality::Unique,
            8 => ItemQuality::Crafted,
            _ => ItemQuality::Normal,
        }
    }
}

/// A total function to map raw 4-bit value to ItemQuality.
/// Verified by Kani to have no panics for any u8 input.
pub fn map_item_quality(v: u8) -> ItemQuality {
    ItemQuality::from(v)
}
