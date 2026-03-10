use crate::item::{Checksum, HuffmanTree, Item};
use std::io;

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
    pub raw_prefix: Vec<u8>,
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
    let nul = field
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(field.len());
    Ok(String::from_utf8_lossy(&field[..nul]).to_string())
}

fn write_u32_le(bytes: &mut [u8], offset: usize, value: u32) -> io::Result<()> {
    let end = offset + 4;
    if end > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Buffer too small for u32 write at offset {offset}."),
        ));
    }
    bytes[offset..end].copy_from_slice(&value.to_le_bytes());
    Ok(())
}

fn write_ascii_nul_padded(
    bytes: &mut [u8],
    offset: usize,
    len: usize,
    value: &str,
) -> io::Result<()> {
    let end = offset + len;
    if end > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Buffer too small for ASCII field at offset {offset}."),
        ));
    }

    if !value.is_ascii() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Character name must be ASCII.",
        ));
    }

    let bytes_value = value.as_bytes();
    if bytes_value.len() >= len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Character name too long (max {} bytes without terminator).",
                len - 1
            ),
        ));
    }

    bytes[offset..end].fill(0);
    bytes[offset..offset + bytes_value.len()].copy_from_slice(bytes_value);
    Ok(())
}

pub fn finalize_save_bytes(bytes: &mut Vec<u8>) -> io::Result<()> {
    if bytes.len() < 16 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Save bytes must be at least 16 bytes to finalize.",
        ));
    }

    let len = bytes.len();
    if len > u32::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Save file is too large to store in u32 file_size.",
        ));
    }

    write_u32_le(bytes, FILE_SIZE_OFFSET, len as u32)?;
    Checksum::fix(bytes);
    Ok(())
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
                format!(
                    "Invalid magic number: expected 0x{:08X}, got 0x{:08X}",
                    D2S_MAGIC, magic
                ),
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
            raw_prefix: bytes[..MIN_HEADER_LEN].to_vec(),
        };

        Ok(Save { header })
    }
}

impl Header {
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bytes = self.raw_prefix.clone();
        if bytes.len() < MIN_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Stored header prefix is too short.",
            ));
        }

        write_u32_le(&mut bytes, MAGIC_OFFSET, self.magic)?;
        write_u32_le(&mut bytes, VERSION_OFFSET, self.version)?;
        write_u32_le(&mut bytes, FILE_SIZE_OFFSET, self.file_size)?;
        write_u32_le(&mut bytes, CHECKSUM_OFFSET, self.checksum)?;
        write_u32_le(&mut bytes, ACTIVE_WEAPON_OFFSET, self.active_weapon)?;

        if CHAR_CLASS_OFFSET >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Header prefix is too short for class.",
            ));
        }
        bytes[CHAR_CLASS_OFFSET] = self.char_class;

        if CHAR_LEVEL_OFFSET >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Header prefix is too short for level.",
            ));
        }
        bytes[CHAR_LEVEL_OFFSET] = self.char_level;

        write_u32_le(&mut bytes, LAST_PLAYED_OFFSET, self.last_played)?;
        write_ascii_nul_padded(&mut bytes, CHAR_NAME_OFFSET, CHAR_NAME_LEN, &self.char_name)?;

        Ok(bytes)
    }
}

impl Save {
    pub fn apply_header_to_bytes(&self, bytes: &mut Vec<u8>) -> io::Result<()> {
        if bytes.len() < MIN_HEADER_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Target buffer is too short to receive header bytes.",
            ));
        }
        let header_bytes = self.header.to_bytes()?;
        bytes[..MIN_HEADER_LEN].copy_from_slice(&header_bytes);
        finalize_save_bytes(bytes)?;
        Ok(())
    }
}

pub fn rebuild_item_section(
    bytes: &[u8],
    items: &[Item],
    huffman: &HuffmanTree,
) -> io::Result<Vec<u8>> {
    let jm_positions = find_jm_markers(bytes);
    if jm_positions.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Save needs at least two JM markers (player & corpse sections)",
        ));
    }

    let jm1 = jm_positions[0];
    let jm2 = jm_positions[1];

    let mut serialized_section = Vec::new();
    for item in items {
        serialized_section.extend_from_slice(&item.to_bytes(huffman)?);
    }

    if items.len() > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "item count exceeds 65535",
        ));
    }
    let count_u16 = items.len() as u16;

    let mut rebuilt =
        Vec::with_capacity(bytes.len() - (jm2 - (jm1 + 4)) + serialized_section.len());
    rebuilt.extend_from_slice(&bytes[..jm1]);
    rebuilt.extend_from_slice(b"JM");
    rebuilt.extend_from_slice(&count_u16.to_le_bytes());
    rebuilt.extend_from_slice(&serialized_section);
    rebuilt.extend_from_slice(&bytes[jm2..]);

    finalize_save_bytes(&mut rebuilt)?;
    Ok(rebuilt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io;

    fn fixture_bytes(path: &str) -> Vec<u8> {
        let repo_root = env!("CARGO_MANIFEST_DIR");
        fs::read(std::path::Path::new(repo_root).join(path)).expect("fixture should exist")
    }

    #[test]
    fn header_to_bytes_matches_original_prefix() -> io::Result<()> {
        let bytes = fixture_bytes("tests/fixtures/savegames/original/amazon_empty.d2s");
        let save = Save::from_bytes(&bytes)?;
        let header_bytes = save.header.to_bytes()?;
        assert_eq!(header_bytes, bytes[..MIN_HEADER_LEN]);
        Ok(())
    }

    #[test]
    fn header_writer_nul_pads_short_name() -> io::Result<()> {
        let bytes = fixture_bytes("tests/fixtures/savegames/original/amazon_empty.d2s");
        let mut save = Save::from_bytes(&bytes)?;
        save.header.char_name = "AMY".into();
        let header_bytes = save.header.to_bytes()?;

        assert_eq!(
            &header_bytes[CHAR_NAME_OFFSET..CHAR_NAME_OFFSET + 4],
            b"AMY\0"
        );
        assert!(
            header_bytes[CHAR_NAME_OFFSET + 4..CHAR_NAME_OFFSET + CHAR_NAME_LEN]
                .iter()
                .all(|&byte| byte == 0)
        );
        Ok(())
    }

    #[test]
    fn apply_header_round_trips_level_and_integrity() -> io::Result<()> {
        let mut bytes = fixture_bytes("tests/fixtures/savegames/original/amazon_empty.d2s");
        let mut save = Save::from_bytes(&bytes)?;
        save.header.char_level = 99;
        save.apply_header_to_bytes(&mut bytes)?;

        let reparsed = Save::from_bytes(&bytes)?;
        assert_eq!(reparsed.header.char_level, 99);
        assert_eq!(reparsed.header.file_size as usize, bytes.len());
        let recalculated = recalculate_checksum(&bytes)?;
        assert_eq!(reparsed.header.checksum, recalculated);
        Ok(())
    }
}
