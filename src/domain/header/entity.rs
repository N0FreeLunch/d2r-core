use crate::domain::item::quality::ItemQuality;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemHeader {
    pub id: Option<u32>,
    pub quality: Option<ItemQuality>,
    pub version: u8,
    pub is_compact: bool,
    pub is_identified: bool,
    pub is_socketed: bool,
    pub is_personalized: bool,
    pub is_runeword: bool,
    pub is_ethereal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderAxiom {
    pub version: u8,
    pub alpha_mode: bool,
}

impl HeaderAxiom {
    pub fn is_plausible(&self, mode: u8, location: u8, code: &str, flags: u32) -> bool {
        let trimmed = code.trim();
        if trimmed.is_empty() { return false; }
        
        // Diablo 2 codes are never right-aligned with a leading space.
        if code.starts_with(' ') {
            return false;
        }

        if self.alpha_mode {
            // Alpha v105 item headers are expected to use the v5/v1 family only.
            if !(self.version == 5 || self.version == 1) {
                return false; 
            }
            if mode > 7 || location > 15 { 
                return false; 
            }
            if (flags & 0xF8000000) != 0 {
                return false;
            }

            // Truth table: ww l, xlp, and buc must be accepted.
            if matches!(trimmed, "ww l" | "xlp" | "buc") {
                return true;
            }

            // For Alpha, we'll rely on the caller to check templates for now,
            // or we can integrate DataRepository here.
            true
        } else {
            if mode > 6 || location > 15 { return false; }
            true
        }
    }
}
