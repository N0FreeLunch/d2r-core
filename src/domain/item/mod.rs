pub mod quality;
pub mod entity;
pub mod serialization;

pub use quality::{ItemQuality, map_item_quality};
pub use entity::{
    ItemHeader, ItemBody, Item, ItemModule, CharmBagData, CursedItemData,
    RecordedBit, ItemBitRange, BitSegment
};
// Removed redundant re-exports: ItemProperty, ItemStats moved to domain::stats
pub use serialization::{BitEmitter, HuffmanTree};
