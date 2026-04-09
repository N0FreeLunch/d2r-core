pub mod quality;
pub mod entity;
pub mod stat_list;
pub mod serialization;

pub use quality::{ItemQuality, map_item_quality};
pub use entity::{
    ItemHeader, ItemBody, Item, ItemModule, CharmBagData, CursedItemData,
    RecordedBit, ItemBitRange, BitSegment
};
pub use crate::domain::stats::{ItemProperty, ItemStats};
pub use serialization::{BitEmitter, HuffmanTree};
