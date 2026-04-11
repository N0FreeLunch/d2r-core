/// Stable D2S Header Constants
/// 
/// These constants define the physical layout of the standard D2S save game header.

pub const D2S_MAGIC: u32 = 0xaa55aa55;

pub const MAGIC_OFFSET: usize = 0;
pub const VERSION_OFFSET: usize = 4;
pub const FILE_SIZE_OFFSET: usize = 8;
pub const CHECKSUM_OFFSET: usize = 12;
pub const ACTIVE_WEAPON_OFFSET: usize = 16;
pub const CHAR_CLASS_OFFSET: usize = 24;
pub const CHAR_LEVEL_OFFSET: usize = 27;
pub const LAST_PLAYED_OFFSET: usize = 32;
pub const CHAR_NAME_OFFSET: usize = 299;
pub const CHAR_NAME_LEN: usize = 48;
pub const ACTIVE_ACT_OFFSET: usize = 21;
pub const PROGRESS_FLAG_OFFSET: usize = 108;

/// Minimum header length required to reach the end of the character name field.
pub const MIN_HEADER_LEN: usize = CHAR_NAME_OFFSET + CHAR_NAME_LEN;
