use crate::data::waypoints::{WaypointEntry, WAYPOINTS};

#[derive(Clone, Copy)]
pub struct Waypoint {
    entry: &'static WaypointEntry,
}

impl std::fmt::Debug for Waypoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Waypoint")
            .field("act", &self.act())
            .field("index", &self.index())
            .field("name", &self.name())
            .field("ws_bit", &self.ws_bit())
            .finish()
    }
}

impl Waypoint {
    pub fn new(entry: &'static WaypointEntry) -> Self {
        Self { entry }
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
}

pub struct WaypointSet {
    waypoints: Vec<Waypoint>,
}

impl WaypointSet {
    pub fn new() -> Self {
        let waypoints = WAYPOINTS.iter().map(Waypoint::new).collect();
        Self { waypoints }
    }

    pub fn waypoints(&self) -> &[Waypoint] {
        &self.waypoints
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
        let waypoint_set = WaypointSet::new();
        assert!(!waypoint_set.waypoints().is_empty());
        
        let act1_town = waypoint_set.find_by_name("Act 1 - Town").expect("Should find Act 1 - Town");
        assert_eq!(act1_town.act(), 1);
        assert_eq!(act1_town.ws_bit(), 0);
    }
}
