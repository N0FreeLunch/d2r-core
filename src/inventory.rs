pub struct InventoryGrid {
    width: u8,
    height: u8,
    grid: [[bool; 10]; 4], // Row-major: [vertical][horizontal]
}

impl InventoryGrid {
    pub fn new() -> Self {
        InventoryGrid {
            width: 10,
            height: 4,
            grid: [[false; 10]; 4],
        }
    }

    /// Marks a rectangle as occupied. Returns false if any cell is already occupied or out of bounds.
    pub fn occupy(&mut self, x: u8, y: u8, w: u8, h: u8) -> bool {
        if x + w > self.width || y + h > self.height {
            return false;
        }

        // Check if all needed cells are free
        for r in y..(y + h) {
            for c in x..(x + w) {
                if self.grid[r as usize][c as usize] {
                    return false;
                }
            }
        }

        // Mark them as occupied
        for r in y..(y + h) {
            for c in x..(x + w) {
                self.grid[r as usize][c as usize] = true;
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
                        if self.grid[ir as usize][ic as usize] {
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
}
