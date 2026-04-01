use crate::data::quests::{QuestEntry, V105_QUESTS};

#[derive(Clone, Copy)]
pub struct Quest {
    entry: &'static QuestEntry,
    state: u16,
}

impl std::fmt::Debug for Quest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quest")
            .field("difficulty", &self.difficulty())
            .field("act", &self.act())
            .field("index", &self.index())
            .field("name", &self.name())
            .field("state", &format_args!("0x{:04X}", self.state))
            .field("v105_offset", &self.v105_offset())
            .finish()
    }
}

impl Quest {
    pub fn new(entry: &'static QuestEntry, state: u16) -> Self {
        Self { entry, state }
    }

    pub fn difficulty(&self) -> u8 {
        self.entry.difficulty
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

    pub fn v105_offset(&self) -> usize {
        self.entry.v105_offset
    }

    pub fn state(&self) -> u16 {
        self.state
    }

    pub fn is_completed(&self) -> bool {
        (self.state & 0x01) != 0
    }

    pub fn set_completed(&mut self, completed: bool) {
        if completed {
            self.state |= 0x01;
            // Also set 0x10 in high byte (seen/checked) as per save.rs logic
            self.state |= 0x1000;
        } else {
            self.state &= !0x01;
            self.state &= !0x1000;
        }
    }
}

pub struct QuestSet {
    quests: Vec<Quest>,
}

impl QuestSet {
    pub fn new_v105_empty() -> Self {
        let quests = V105_QUESTS.iter().map(|e| Quest::new(e, 0)).collect();
        Self { quests }
    }

    pub fn from_v105_bytes(bytes: &[u8]) -> Self {
        let quests = V105_QUESTS
            .iter()
            .map(|entry| {
                let offset = entry.v105_offset - 403;
                let state = if offset + 1 < bytes.len() {
                    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
                } else {
                    0
                };
                Quest::new(entry, state)
            })
            .collect();
        Self { quests }
    }

    pub fn sync_to_v105_bytes(&self, bytes: &mut [u8]) {
        for quest in &self.quests {
            let offset = quest.v105_offset() - 403;
            if offset + 1 < bytes.len() {
                let le_bytes = quest.state().to_le_bytes();
                bytes[offset] = le_bytes[0];
                bytes[offset + 1] = le_bytes[1];
            }
        }
    }

    pub fn quests(&self) -> &[Quest] {
        &self.quests
    }

    pub fn quests_mut(&mut self) -> &mut [Quest] {
        &mut self.quests
    }

    pub fn find_by_name(&self, name: &str) -> Option<Quest> {
        self.quests.iter().find(|q| q.name() == name).copied()
    }

    pub fn filter_by_difficulty(&self, difficulty: u8) -> Vec<Quest> {
        self.quests
            .iter()
            .filter(|q| q.difficulty() == difficulty)
            .copied()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_set_initialization() {
        let quest_set = QuestSet::new_v105_empty();
        assert!(!quest_set.quests().is_empty());
        
        let den = quest_set.find_by_name("Den of Evil").expect("Should find Den of Evil");
        assert_eq!(den.difficulty(), 0);
        assert_eq!(den.act(), 1);
        assert!(!den.is_completed());
    }
}
