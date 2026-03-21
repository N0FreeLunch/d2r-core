//! Value Object (VO) layer.
//! This module defines opaque types that encapsulate core domain invariants.
//! These types will later be shared with Elm via `elm-rs`.

/// Opaque wrapper for Item Stat Values.
/// Protects against invalid stat boundaries and mutations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStatValue(i32);

impl ItemStatValue {
    pub fn new(val: i32) -> Result<Self, &'static str> {
        // Example invariant check placeholder
        if val < -100_000 || val > 100_000 {
            return Err("Item stat value out of safe boundaries");
        }
        Ok(Self(val))
    }

    pub fn value(&self) -> i32 {
        self.0
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
    pub fn new(x: u8, y: u8) -> Result<Self, &'static str> {
        // Example grid boundaries for typical Diablo 2 inventory
        if x > 10 || y > 10 {
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
