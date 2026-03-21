//! Value Object (VO) layer.
//! This module defines opaque types that encapsulate core domain invariants.
//! These types will later be shared with Elm via `elm-rs`.

/// Opaque wrapper for Item Stat Values.
/// Protects against invalid stat boundaries and mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStatValue(i32);

impl ItemStatValue {
    pub const MIN_SAFE: i32 = -100_000;
    pub const MAX_SAFE: i32 = 100_000;

    pub fn new(val: i32) -> Result<Self, &'static str> {
        if val < Self::MIN_SAFE || val > Self::MAX_SAFE {
            return Err("Item stat value out of safe boundaries");
        }
        Ok(Self(val))
    }

    pub fn value(&self) -> i32 {
        self.0
    }
}

impl TryFrom<i32> for ItemStatValue {
    type Error = &'static str;
    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// Opaque wrapper for Inventory Coordinates.
/// Ensures valid grid placement bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventoryCoordinate {
    x: u8,
    y: u8,
}

impl InventoryCoordinate {
    pub const MAX_X: u8 = 10;
    pub const MAX_Y: u8 = 10;

    pub fn new(x: u8, y: u8) -> Result<Self, &'static str> {
        if x > Self::MAX_X || y > Self::MAX_Y {
            return Err("Inventory coordinates out of bounds");
        }
        Ok(Self { x, y })
    }

    pub fn x(&self) -> u8 {
        self.x
    }

    pub fn y(&self) -> u8 {
        self.y
    }
}

impl TryFrom<(u8, u8)> for InventoryCoordinate {
    type Error = &'static str;
    fn try_from(value: (u8, u8)) -> Result<Self, Self::Error> {
        Self::new(value.0, value.1)
    }
}

