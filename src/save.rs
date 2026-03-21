use std::io;

pub const D2S_MAGIC: u32 = 0xaa55aa55;

pub const ACTIVE_WEAPON_OFFSET: usize = 16;
pub const CHAR_CLASS_OFFSET: usize = 24;
pub const CHAR_LEVEL_OFFSET: usize = 27;
pub const LAST_PLAYED_OFFSET: usize = 32;
pub const CHAR_NAME_OFFSET: usize = 299;
pub const CHAR_NAME_LEN: usize = 48;

const MIN_HEADER_LEN: usize = CHAR_NAME_OFFSET + CHAR_NAME_LEN;

#[derive(Debug)]
pub struct Header {
    pub magic: u32,
    pub version: u32,
    pub file_size: u32,
    pub checksum: u32,
    pub active_weapon: u32,
    pub char_name: String,
    pub char_class: u8,
    pub char_level: u8,
    pub last_played: u32,
}

#[derive(Debug)]
pub struct Save {
    pub header: Header,
}

pub fn class_name(class_id: u8) -> &'static str {
    match class_id {
        0 => "Amazon",
        1 => "Sorceress",
        2 => "Necromancer",
        3 => "Paladin",
        4 => "Barbarian",
        5 => "Druid",
        6 => "Assassin",
        _ => "Unknown",
    }
}

pub fn find_jm_markers(bytes: &[u8]) -> Vec<usize> {
    let mut jm_positions = Vec::new();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'J' && bytes[i + 1] == b'M' {
            jm_positions.push(i);
        }
    }
    jm_positions
}

pub fn recalculate_checksum(bytes: &[u8]) -> io::Result<u32> {
    if bytes.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Save file is too small for checksum recalculation.",
        ));
    }

    let mut calc_bytes = bytes.to_vec();
    calc_bytes[12..16].copy_from_slice(&[0, 0, 0, 0]);
    Ok(crate::item::Checksum::calculate(&calc_bytes) as u32)
}

fn parse_ascii_field(bytes: &[u8], offset: usize, len: usize) -> io::Result<String> {
    let end = offset + len;
    if bytes.len() < end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Save file is too small for ASCII field at offset {offset}."),
        ));
    }

    let field = &bytes[offset..end];
    let nul = field.iter().position(|&byte| byte == 0).unwrap_or(field.len());
    Ok(String::from_utf8_lossy(&field[..nul]).to_string())
}

impl Save {
    pub fn from_bytes(bytes: &[u8]) -> io::Result<Self> {
        if bytes.len() < MIN_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Save file is too small for a D2R header ({MIN_HEADER_LEN} bytes)."),
            ));
        }

        let magic = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
        if magic != D2S_MAGIC {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid magic number: expected 0x{:08X}, got 0x{:08X}", D2S_MAGIC, magic),
            ));
        }

        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let file_size = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let checksum = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let active_weapon = u32::from_le_bytes(
            bytes[ACTIVE_WEAPON_OFFSET..ACTIVE_WEAPON_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let char_class = bytes[CHAR_CLASS_OFFSET];
        let char_level = bytes[CHAR_LEVEL_OFFSET];
        let last_played = u32::from_le_bytes(
            bytes[LAST_PLAYED_OFFSET..LAST_PLAYED_OFFSET + 4]
                .try_into()
                .unwrap(),
        );
        let char_name = parse_ascii_field(bytes, CHAR_NAME_OFFSET, CHAR_NAME_LEN)?;

        let header = Header {
            magic,
            version,
            file_size,
            checksum,
            active_weapon,
            char_name,
            char_class,
            char_level,
            last_played,
        };

        Ok(Save { header })
    }
}
