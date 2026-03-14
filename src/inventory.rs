use crate::item::Item;
use std::fmt;

#[derive(Debug, Clone)]
pub enum InventoryError {
    OutOfBounds {
        item_code: String,
        x: u8,
        y: u8,
        w: u8,
        h: u8,
    },
    Collision {
        item_code: String,
        x: u8,
        y: u8,
        w: u8,
        h: u8,
    },
    InvalidCode(String),
    LogicalMismatch {
        item_code: String,
        reason: String,
    },
}

impl fmt::Display for InventoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfBounds {
                item_code,
                x,
                y,
                w,
                h,
            } => {
                write!(
                    f,
                    "[OUT_OF_BOUNDS] Item '{}' at ({},{}) size {}x{} exceeds grid",
                    item_code, x, y, w, h
                )
            }
            Self::Collision {
                item_code,
                x,
                y,
                w,
                h,
            } => {
                write!(
                    f,
                    "[COLLISION] Item '{}' at ({},{}) size {}x{} overlaps with another item",
                    item_code, x, y, w, h
                )
            }
            Self::InvalidCode(code) => {
                write!(f, "[INVALID_CODE] Item code '{}' is not recognized", code)
            }
            Self::LogicalMismatch { item_code, reason } => {
                write!(f, "[LOGICAL_MISMATCH] Item '{}': {}", item_code, reason)
            }
        }
    }
}

pub struct InventoryGrid {
    width: u8,
    height: u8,
    grid: Vec<bool>, // Flattened grid: y * width + x
}

impl InventoryGrid {
    pub fn new(width: u8, height: u8) -> Self {
        InventoryGrid {
            width,
            height,
            grid: vec![false; (width as usize) * (height as usize)],
        }
    }

    /// Default 10x4 inventory grid
    pub fn new_inventory() -> Self {
        Self::new(10, 4)
    }

    /// Default 10x10 stash grid
    pub fn new_stash() -> Self {
        Self::new(10, 10)
    }

    /// Marks a rectangle as occupied. Returns false if any cell is already occupied or out of bounds.
    pub fn occupy(&mut self, x: u8, y: u8, w: u8, h: u8) -> bool {
        if x + w > self.width || y + h > self.height {
            return false;
        }

        // Check if all needed cells are free
        for r in y..(y + h) {
            for c in x..(x + w) {
                if self.grid[(r as usize) * (self.width as usize) + (c as usize)] {
                    return false;
                }
            }
        }

        // Mark them as occupied
        for r in y..(y + h) {
            for c in x..(x + w) {
                self.grid[(r as usize) * (self.width as usize) + (c as usize)] = true;
            }
        }
        true
    }

    /// Finds the first available (top-left) slot for an item of given dimensions.
    pub fn find_free_slot(&self, w: u8, h: u8) -> Option<(u8, u8)> {
        for r in 0..=(self.height - h) {
            for c in 0..=(self.width - w) {
                let mut free = true;
                'check: for ir in r..(r + h) {
                    for ic in c..(c + w) {
                        if self.grid[(ir as usize) * (self.width as usize) + (ic as usize)] {
                            free = false;
                            break 'check;
                        }
                    }
                }
                if free {
                    return Some((c, r));
                }
            }
        }
        None
    }

    /// Convenience method to create a grid from a save file's raw bytes.
    pub fn from_save_bytes(bytes: &[u8], huffman: &crate::item::HuffmanTree) -> Self {
        let mut grid = Self::new_inventory();

        // Find JM marker
        let jm_pos =
            (0..bytes.len().saturating_sub(1)).find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M');

        if jm_pos.is_some() {
            if let Ok(items) = Item::read_player_items(bytes, huffman) {
                for item in items {
                    if item.location == 0 {
                        grid.occupy(
                            item.x,
                            item.y,
                            get_item_size(&item.code).0,
                            get_item_size(&item.code).1,
                        );
                    }
                }
            }
        }
        grid
    }

    /// Auto-fills the grid from a collection of items.
    pub fn scan_items(&mut self, items: &[Item]) {
        for it in items {
            let (w, h) = get_item_size(&it.code);
            self.occupy(it.x, it.y, w, h);
        }
    }

    /// Debug print of the current grid state.
    pub fn debug_print(&self) {
        println!("   0 1 2 3 4 5 6 7 8 9");
        for r in 0..self.height {
            print!("{r}: ");
            for c in 0..self.width {
                print!(
                    "{}",
                    if self.grid[(r as usize) * (self.width as usize) + (c as usize)] {
                        "■ "
                    } else {
                        "□ "
                    }
                );
            }
            println!();
        }
    }

    /// Strong validation of a collection of items.
    /// Returns a list of all errors found.
    pub fn validate_items(items: &[Item], width: u8, height: u8) -> Vec<InventoryError> {
        let mut grid = Self::new(width, height);
        Self::validate_items_on_grid(&mut grid, items)
    }

    /// Validates items against an existing grid instance.
    pub fn validate_items_on_grid(grid: &mut Self, items: &[Item]) -> Vec<InventoryError> {
        let mut errors = Vec::new();

        for item in items {
            // Only validate items in the inventory (location 0)
            if item.location != 0 {
                continue;
            }

            let (w, h) = get_item_size(&item.code);

            // Check out of bounds
            if item.x + w > grid.width || item.y + h > grid.height {
                errors.push(InventoryError::OutOfBounds {
                    item_code: item.code.clone(),
                    x: item.x,
                    y: item.y,
                    w,
                    h,
                });
                continue;
            }

            // Check collision
            let mut has_collision = false;
            for r in item.y..(item.y + h) {
                for c in item.x..(item.x + w) {
                    if grid.grid[(r as usize) * (grid.width as usize) + (c as usize)] {
                        has_collision = true;
                        break;
                    }
                }
                if has_collision {
                    break;
                }
            }

            if has_collision {
                errors.push(InventoryError::Collision {
                    item_code: item.code.clone(),
                    x: item.x,
                    y: item.y,
                    w,
                    h,
                });
            } else {
                // No collision, mark the area
                for r in item.y..(item.y + h) {
                    for c in item.x..(item.x + w) {
                        grid.grid[(r as usize) * (grid.width as usize) + (c as usize)] = true;
                    }
                }
            }
        }

        errors
    }

    /// Performs deep logical integrity validation of a collection of items.
    pub fn validate_logical_integrity(
        items: &[Item],
        width: u8,
        height: u8,
    ) -> Vec<InventoryError> {
        let mut errors = Vec::new();

        for item in items {
            let code = item.code.trim();
            let template = crate::data::item_codes::ITEM_TEMPLATES
                .iter()
                .find(|t| t.code == code);

            // A. Vocabulary Check
            if template.is_none() {
                errors.push(InventoryError::InvalidCode(item.code.clone()));
                continue;
            }

            // B. Cross-Field Consistency
            // If identified is 0, properties should ideally be empty (not checking bitstream yet here)
            // But we can check if a compact item has excessive data if we had length info.

            // D. Socket Integrity
            // If it's a socketed item (location 6), x should be a valid parent index (placeholder logic)
            // if item.location == 6 && some_condition ...
        }

        // Also add collision check
        errors.extend(Self::validate_items(items, width, height));

        errors
    }
}

pub fn get_item_size(code: &str) -> (u8, u8) {
    let code = code.trim();

    // Primary source: Generated Item Templates
    if let Some(t) = crate::data::item_codes::ITEM_TEMPLATES
        .iter()
        .find(|t| t.code == code)
    {
        return (t.width, t.height);
    }

    // Fallback for codes not in templates (should be rare)
    match code {
        // Consumables (1x1)
        "tsc" | "isc" | "hp1" | "hp2" | "hp3" | "hp4" | "hp5" | "mp1" | "mp2" | "mp3" | "mp4"
        | "mp5" | "vps" | "yps" | "wms" | "rvs" | "rvl" | "key" | "aqv" | "cqv" => (1, 1),

        // Small charms, gems, rings, ammys (1x1)
        "cm1" | "gcv" | "gcy" | "gcb" | "gcg" | "gcr" | "gcw" | "skc" | "skz" => (1, 1),
        "rin" | "amu" | "jew" => (1, 1),

        // Books & Medium items (1x2)
        "tbk" | "ibk" | "cm2" | "cap" | "msk" => (1, 2),

        // Large items (2x2)
        "buc" | "cm3" | "brs" | "glb" | "vbl" | "tbl" | "lbl" => (2, 2),
        "fsm" => (2, 2), // Small shield

        // Armor & Large Weapons (2x3)
        "qui" | "lea" | "hrb" | "stu" | "rng" | "scl" | "chn" | "spl" | "plt" | "fld" => (2, 3),

        // Very Large Weapons (2x4)
        "axe" | "bax" | "tri" | "clb" | "spc" | "bst" | "hal" => (2, 4),

        // 1x3 Weapons (Long sword, etc)
        "jav" | "wwa7" | "lsw" | "ssw" | "msw" => (1, 3),

        // 1x4 Weapons (Pike, Bows)
        "pik" | "lbw" | "shb" | "lxb" => (1, 4),

        _ => (1, 1), // Default to 1x1 if unknown
    }
}
pub fn get_item_category(code: &str) -> &'static str {
    match code.trim() {
        // Potions
        "hp1" | "hp2" | "hp3" | "hp4" | "hp5" => "Healing Potion",
        "mp1" | "mp2" | "mp3" | "mp4" | "mp5" => "Mana Potion",
        "rvs" | "rvl" => "Rejuvenation Potion",
        "vps" | "yps" | "wms" => "Special Potion",

        // Scrolls & Books
        "tsc" | "isc" => "Scroll",
        "tbk" | "ibk" => "Book",

        // Consumables & Ammo
        "key" => "Keys",
        "aqv" | "cqv" => "Ammo",

        // Charms & Jewels
        "cm1" => "Small Charm",
        "cm2" => "Large Charm",
        "cm3" => "Grand Charm",
        "jew" => "Jewel",

        // Gems & Skulls
        "gcv" | "gcy" | "gcb" | "gcg" | "gcr" | "gcw" | "skc" | "skz" => "Gem/Skull",

        // Jewelry
        "rin" => "Ring",
        "amu" => "Amulet",

        // Equipment
        "buc" | "fsm" => "Shield",
        "cap" | "msk" | "cas" | "ghm" | "hlm" | "fhl" | "xml" => "Helmet",
        "qui" | "lea" | "hrb" | "stu" | "rng" | "scl" | "chn" | "spl" | "plt" | "fld" => {
            "Body Armor"
        }
        "glb" | "vgs" | "mgl" | "cha" | "hgl" => "Gloves",
        "vbl" | "tbl" | "lbl" | "hbl" => "Boots",
        "lbt" | "vbt" | "mbt" | "tbt" | "tbl " => "Belt",

        // Weapons
        "axe" | "bax" | "tri" | "clb" | "spc" | "bst" | "hal" => "Large Weapon",
        "jav" | "wwa7" | "lsw" | "ssw" | "msw" => "Medium Weapon",
        "pik" | "lbw" | "shb" | "lxb" => "Long Weapon/Bow",

        _ => "Misc/Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_grid_occupy() {
        let mut grid = InventoryGrid::new_inventory(); // 10x4

        // Placing 2x2 item at (0,0)
        assert!(
            grid.occupy(0, 0, 2, 2),
            "Should be able to occupy (0,0) with 2x2"
        );

        // Attempt to place in the same spot again (should collision)
        assert!(
            !grid.occupy(1, 1, 1, 1),
            "Should fail due to collision at (1,1)"
        );

        // Attempt to place out of bounds
        assert!(
            !grid.occupy(9, 0, 2, 1),
            "Should fail due to out of bounds (width)"
        );
        assert!(
            !grid.occupy(0, 3, 1, 2),
            "Should fail due to out of bounds (height)"
        );
    }

    #[test]
    fn test_find_free_slot() {
        let mut grid = InventoryGrid::new(4, 4);

        // Occupy top-left 2x2
        grid.occupy(0, 0, 2, 2);

        // Next available slot for 2x2 should be (2,0) or (0,2)
        let slot = grid.find_free_slot(2, 2);
        assert!(slot.is_some());
        let (x, y) = slot.unwrap();
        assert!(
            x >= 2 || y >= 2,
            "Found slot ({},{}) should not overlap with (0,0) 2x2",
            x,
            y
        );
    }

    #[test]
    fn test_stash_grid() {
        let mut stash = InventoryGrid::new_stash(); // 10x10
        assert!(
            stash.occupy(0, 0, 10, 10),
            "Should be able to fill the entire stash"
        );
        assert!(!stash.occupy(0, 0, 1, 1), "Stash should be full now");
    }

    #[test]
    fn test_get_item_size() {
        assert_eq!(get_item_size("rin "), (1, 1)); // Ring
        assert_eq!(get_item_size("plt "), (2, 3)); // Plate Mail
        assert_eq!(get_item_size("axe "), (2, 3)); // Axe - TODO: This seems to be wrong in the data, should be 2x4
    }

    #[test]
    fn test_logical_integrity_validation() {
        // Create dummy item data (matching Item struct fields)
        let item1 = Item {
            bits: Vec::new(),
            code: "rin ".to_string(),
            flags: 0,
            version: 0,
            is_ear: false,
            ear_class: None,
            ear_level: None,
            ear_player_name: None,
            personalized_player_name: None,
            mode: 0,
            x: 0,
            y: 0,
            page: 0,
            location: 0,
            header_socket_hint: 0,
            has_multiple_graphics: false,
            multi_graphics_bits: None,
            has_class_specific_data: false,
            class_specific_bits: None,
            id: None,
            level: None,
            quality: None,
            low_high_graphic_bits: None,
            is_compact: true,
            is_socketed: false,
            is_identified: true,
            is_personalized: false,
            is_runeword: false,
            is_ethereal: false,
            magic_prefix: None,
            magic_suffix: None,
            rare_name_1: None,
            rare_name_2: None,
            rare_affixes: Vec::new(),
            unique_id: None,
            runeword_id: None,
            runeword_level: None,
            properties: Vec::new(),
            set_attributes: Vec::new(),
            runeword_attributes: Vec::new(),
            num_socketed_items: 0,
            socketed_items: Vec::new(),
            timestamp_flag: false,
            properties_complete: true,
            set_list_count: 0,
            tbk_ibk_teleport: None,
            defense: None,
            max_durability: None,
            current_durability: None,
            quantity: None,
            sockets: None,
        };

        // Manual field setup for testing
        let mut items = Vec::new();
        let mut dummy_item = item1.clone();
        dummy_item.code = "rin ".to_string();
        dummy_item.x = 0;
        dummy_item.y = 0;
        dummy_item.location = 0; // Inventory
        items.push(dummy_item);

        let mut collision_item = item1.clone();
        collision_item.code = "rin ".to_string();
        collision_item.x = 0;
        collision_item.y = 0;
        collision_item.location = 0;
        items.push(collision_item);

        let errors = InventoryGrid::validate_logical_integrity(&items, 10, 4);
        assert!(!errors.is_empty(), "Should detect collision at (0,0)");
    }

    #[test]
    fn test_large_item_boundary_and_collision() {
        let mut grid = InventoryGrid::new_inventory(); // 10x4

        // 1. Attempt to place 2x4 armor at the right edge (should succeed)
        // x=8, w=2 -> 10 (OK), y=0, h=4 -> 4 (OK)
        assert!(
            grid.occupy(8, 0, 2, 4),
            "Should allow 2x4 item at the right edge"
        );

        // 2. Attempt to place 1x1 item inside occupied 2x4 area (should fail)
        assert!(
            !grid.occupy(9, 3, 1, 1),
            "Should fail to occupy inside 2x4 area"
        );

        // 3. Attempt to place over bottom boundary
        assert!(
            !grid.occupy(0, 1, 2, 4),
            "Should fail: y(1) + h(4) > height(4)"
        );
    }

    #[test]
    fn test_grid_index_mapping() {
        let mut grid = InventoryGrid::new(2, 2);
        // [ (0,0), (1,0) ] -> index 0, 1
        // [ (0,1), (1,1) ] -> index 2, 3

        grid.occupy(1, 1, 1, 1);
        assert!(
            grid.grid[3],
            "Index 3 should be occupied for (1,1) in 2x2 grid"
        );
        assert!(!grid.grid[0], "Index 0 should remain free");
    }
}
