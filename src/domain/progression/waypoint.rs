use crate::data::waypoints::{WaypointEntry, WAYPOINTS};

#[derive(Clone, Copy)]
pub struct Waypoint {
    entry: &'static WaypointEntry,
    active: bool,
}

impl std::fmt::Debug for Waypoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Waypoint")
            .field("act", &self.act())
            .field("index", &self.index())
            .field("name", &self.name())
            .field("active", &self.active)
            .field("ws_bit", &self.ws_bit())
            .finish()
    }
}

impl Waypoint {
    pub fn new(entry: &'static WaypointEntry, active: bool) -> Self {
        Self { entry, active }
    }

    pub fn act(&self) -> u8 {
        self.entry.act
    }

    pub fn index(&self) -> u8 {
        self.entry.index
    }

    pub fn name(&self) -> &'static str {
        self.entry.name
    }

    pub fn ws_bit(&self) -> u8 {
        self.entry.ws_bit
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }
}

pub struct WaypointSet {
    difficulty: u8,
    waypoints: Vec<Waypoint>,
}

impl WaypointSet {
    pub fn new_empty(difficulty: u8) -> Self {
        let waypoints = WAYPOINTS.iter().map(|e| Waypoint::new(e, false)).collect();
        Self {
            difficulty,
            waypoints,
        }
    }

    pub fn from_bytes(bytes: &[u8], difficulty: u8) -> Self {
        let waypoints = WAYPOINTS
            .iter()
            .map(|entry| {
                // Formula from src/save.rs:
                // raw_bytes[0..8]: "WS" Header
                // Normal difficulty starts at bit 80 (byte 10)
                let global_bit_idx = (8 * 8) + (difficulty as usize * 24 * 8) + (2 * 8) + entry.ws_bit as usize;
                let byte_idx = global_bit_idx / 8;
                let bit_in_byte = global_bit_idx % 8;
                
                let active = if byte_idx < bytes.len() {
                    (bytes[byte_idx] & (1 << bit_in_byte)) != 0
                } else {
                    false
                };
                Waypoint::new(entry, active)
            })
            .collect();
        Self {
            difficulty,
            waypoints,
        }
    }

    pub fn sync_to_bytes(&self, bytes: &mut [u8]) {
        for wp in &self.waypoints {
            let global_bit_idx = (8 * 8) + (self.difficulty as usize * 24 * 8) + (2 * 8) + wp.ws_bit() as usize;
            let byte_idx = global_bit_idx / 8;
            let bit_in_byte = global_bit_idx % 8;
            
            if byte_idx < bytes.len() {
                if wp.is_active() {
                    bytes[byte_idx] |= 1 << bit_in_byte;
                } else {
                    bytes[byte_idx] &= !(1 << bit_in_byte);
                }
            }
        }
    }

    pub fn difficulty(&self) -> u8 {
        self.difficulty
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
    }

    pub fn waypoints_mut(&mut self) -> &mut [Waypoint] {
        &mut self.waypoints
    }

    pub fn find_by_name(&self, name: &str) -> Option<Waypoint> {
        self.waypoints.iter().find(|w| w.name() == name).copied()
    }

    pub fn filter_by_act(&self, act: u8) -> Vec<Waypoint> {
        self.waypoints
            .iter()
            .filter(|w| w.act() == act)
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waypoint_set_initialization() {
        let waypoint_set = WaypointSet::new_empty(0);
        assert!(!waypoint_set.waypoints().is_empty());
        
        let act1_town = waypoint_set.find_by_name("Act 1 - Town").expect("Should find Act 1 - Town");
        assert_eq!(act1_town.act(), 1);
        assert_eq!(act1_town.ws_bit(), 0);
        assert!(!act1_town.is_active());
    }
}
