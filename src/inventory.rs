//! Compatibility bridge for the inventory domain.
//!
//! This module now re-exports logic from `crate::domain::inventory`.
//! New code should prefer importing from `crate::domain::inventory`.

pub use crate::domain::inventory::{
    get_item_category, get_item_size, InventoryError, InventoryGrid,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// Bridge test to ensure the move didn't break basic visibility or behavior
    #[test]
    fn test_bridge_visibility() {
        let grid = InventoryGrid::new_inventory();
        assert_eq!(grid.find_free_slot(2, 2), Some((0, 0)));
    }
}
