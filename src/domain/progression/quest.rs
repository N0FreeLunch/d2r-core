use crate::data::quests::{QuestEntry, V105_QUESTS};

#[derive(Clone, Copy)]
pub struct Quest {
    entry: &'static QuestEntry,
}

impl std::fmt::Debug for Quest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Quest")
            .field("difficulty", &self.difficulty())
            .field("act", &self.act())
            .field("index", &self.index())
            .field("name", &self.name())
            .field("v105_offset", &self.v105_offset())
            .finish()
    }
}

impl Quest {
    pub fn new(entry: &'static QuestEntry) -> Self {
        Self { entry }
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
}

pub struct QuestSet {
    quests: Vec<Quest>,
}

impl QuestSet {
    pub fn new_v105() -> Self {
        let quests = V105_QUESTS.iter().map(Quest::new).collect();
        Self { quests }
    }

    pub fn quests(&self) -> &[Quest] {
        &self.quests
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
        let quest_set = QuestSet::new_v105();
        assert!(!quest_set.quests().is_empty());
        
        let den = quest_set.find_by_name("Den of Evil").expect("Should find Den of Evil");
        assert_eq!(den.difficulty(), 0);
        assert_eq!(den.act(), 1);
    }
}
