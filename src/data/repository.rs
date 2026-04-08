use crate::data::item_codes::ItemTemplate;
use crate::data::item_specs::Runeword;
use crate::data::runes::RuneData;
use crate::data::{item_codes, runewords, runes};

/// A central repository for accessing static game data.
/// This provides a clean interface for domain slices to retrieve templates,
/// stats, and other metadata without direct dependency on the generated modules.
pub struct DataRepository;

impl DataRepository {
    /// Returns the item template for the given item code.
    pub fn get_item_template(code: &str) -> Option<&'static ItemTemplate> {
        item_codes::ITEM_TEMPLATES.iter().find(|t| t.code == code)
    }

    /// Returns the runeword data for the given runeword ID or name.
    /// This uses the `runewords::RUNEWORDS` list.
    pub fn get_runeword(name: &str) -> Option<&'static Runeword> {
        runewords::RUNEWORDS.iter().find(|rw| rw.name == name)
    }

    /// Returns the rune mapping data for a given runeword name from `runes::RUNES`.
    /// Note: In the current data structure, `runes::RUNES` often contains the
    /// sequence of runes (r01, r02...) for a runeword.
    pub fn get_runeword_runes(name: &str) -> Option<&'static RuneData> {
        runes::RUNES.iter().find(|r| r.name == name)
    }
}
