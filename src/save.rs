// Copyright 2026 N0FreeLunch (https://github.com/N0FreeLunch/d2r-core)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::data::stat_costs::{StatCostData, STAT_COSTS};
use crate::item::{Checksum, HuffmanTree, Item};
use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use std::io::{self, Cursor};
use std::mem;

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
pub const SKILL_SECTION_LEN: usize = 30;

fn find_marker(bytes: &[u8], first: u8, second: u8) -> Option<usize> {
    (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == first && bytes[i + 1] == second)
}

#[derive(Debug, Clone)]
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
    pub quests: Option<QuestSection>,
    pub waypoints: Option<WaypointSection>,
    pub expansion: Option<ExpansionSection>,
}

#[derive(Debug, Clone)]
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

#[derive(Debug)]
pub struct SaveSectionMap {
    pub gf_pos: usize,
    pub if_pos: usize,
    pub jm_positions: Vec<usize>,
}

impl SaveSectionMap {
    pub fn first_jm(&self) -> usize {
        *self.jm_positions.first().expect("jm_positions is non-empty")
    }
}

pub fn map_core_sections(bytes: &[u8]) -> io::Result<SaveSectionMap> {
    let gf_pos = find_marker(bytes, b'g', b'f').ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "gf marker not found in save file")
    })?;
    let if_pos = find_marker(bytes, b'i', b'f').ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "if marker not found in save file")
    })?;
    let jm_positions = find_jm_markers(bytes);
    if jm_positions.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No JM markers found in save file",
        ));
    }
    if !(gf_pos < if_pos && if_pos < jm_positions[0]) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Save markers are out of order",
        ));
    }
    Ok(SaveSectionMap {
        gf_pos,
        if_pos,
        jm_positions,
    })
}

pub fn gf_payload_range(map: &SaveSectionMap) -> std::ops::Range<usize> {
    let start = map.gf_pos + 2;
    start..map.if_pos
}

fn stat_cost(stat_id: u32) -> Option<&'static StatCostData> {
    STAT_COSTS.iter().find(|stat| stat.id == stat_id)
}

fn read_bits_dynamic(
    reader: &mut BitReader<Cursor<&[u8]>, LittleEndian>,
    count: u32,
) -> io::Result<u32> {
    let mut value = 0;
    for i in 0..count {
        if reader.read_bit()? {
            value |= 1 << i;
        }
    }
    Ok(value)
}

fn write_bits_dynamic<W: BitWrite>(
    writer: &mut W,
    count: u32,
    value: u32,
) -> io::Result<()> {
    for i in 0..count {
        writer.write_bit((value >> i) & 1 != 0)?;
    }
    Ok(())
}

#[derive(Clone)]
pub struct AttributeEntry {
    pub stat_id: u32,
    pub param: u32,
    pub raw_value: u32,
    pub opaque_bits: Option<Vec<bool>>,
}

#[derive(Clone)]
pub struct AttributeSection {
    pub entries: Vec<AttributeEntry>,
    pub raw_bytes: Vec<u8>,
}

impl AttributeSection {
    pub fn parse(bytes: &[u8], map: &SaveSectionMap) -> io::Result<Self> {
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
        let is_alpha = version == 105 || version == 0x69; // 105 is dec, 0x69 is hex
        let payload_range = gf_payload_range(map);
        let mut reader = BitReader::endian(
            Cursor::new(&bytes[payload_range.start..payload_range.end]),
            LittleEndian,
        );
        let raw_bytes = bytes[map.gf_pos..map.if_pos].to_vec();
        let mut entries = Vec::new();
        let total_bits = ((payload_range.end - payload_range.start) * 8) as u64;
        loop {
            let pos = reader.position_in_bits()?;
            if total_bits.saturating_sub(pos) < 9 {
                break;
            }
            let stat_id = reader.read::<9, u32>()?;
            if stat_id == 0x1FF {
                break;
            }
            let cost = stat_cost(stat_id);
            if let Some(cost) = cost {
                let remaining = total_bits.saturating_sub(reader.position_in_bits()?);
                if (cost.save_param_bits as u64) > remaining {
                    break;
                }
                let param = if cost.save_param_bits > 0 {
                    read_bits_dynamic(&mut reader, cost.save_param_bits as u32)?
                } else {
                    0
                };
                let save_bits = char_stat_save_bits(stat_id, is_alpha);
                let remaining = total_bits.saturating_sub(reader.position_in_bits()?);
                if (save_bits as u64) > remaining {
                    break;
                }
                let raw_value = if save_bits > 0 {
                    read_bits_dynamic(&mut reader, save_bits as u32)?
                } else {
                    0
                };
                entries.push(AttributeEntry {
                    stat_id,
                    param,
                    raw_value,
                    opaque_bits: None,
                });
            } else {
                // Unknown stat ID: collect remaining bits as opaque block
                let mut bits = Vec::new();
                while let Ok(bit) = reader.read_bit() {
                    bits.push(bit);
                }
                entries.push(AttributeEntry {
                    stat_id,
                    param: 0,
                    raw_value: 0,
                    opaque_bits: Some(bits),
                });
                break; // After one opaque block, we stop because we don't know the next stat boundary
            }
        }
        Ok(AttributeSection { entries, raw_bytes })
    }

    pub fn to_bytes(&self, is_alpha: bool) -> io::Result<Vec<u8>> {
        self.to_bytes_from_entries(is_alpha)
    }

    pub fn to_bytes_from_entries(&self, is_alpha: bool) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        let mut writer = BitWriter::endian(&mut buf, LittleEndian);

        // 'gf' marker (2 bytes)
        write_bits_dynamic(&mut writer, 8, b'g' as u32)?;
        write_bits_dynamic(&mut writer, 8, b'f' as u32)?;

        for entry in &self.entries {
            // Alpha v5 Research Mode: Guards disabled for fuzzing.
            if is_alpha && entry.stat_id >= 512 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Alpha v105: ID {} exceeds 9-bit space.", entry.stat_id)
                ));
            }

            if let Some(ref bits) = entry.opaque_bits {
                // Alpha v5 Research Mode: Width guards disabled.
                
                write_bits_dynamic(&mut writer, 9, entry.stat_id)?;
                for &bit in bits {
                    writer.write_bit(bit)?;
                }
                continue;
            }
            let bits = char_stat_save_bits(entry.stat_id, is_alpha);
            if bits == 0 {
                continue;
            }
            write_bits_dynamic(&mut writer, 9, entry.stat_id)?;
            write_bits_dynamic(&mut writer, bits, entry.raw_value)?;
        }
        // 0x1FF terminator
        write_bits_dynamic(&mut writer, 9, 0x1FFu32)?;
        writer.byte_align()?;
        Ok(buf)
    }

    pub fn set_raw(&mut self, stat_id: u32, raw_value: u32) {
        if let Some(e) = self.entries.iter_mut().find(|e| e.stat_id == stat_id) {
            e.raw_value = raw_value;
        } else {
            // If not found, we could push it, but for character stats they should usually exist.
            // Following mini-spec's simple implementation first.
            self.entries.push(AttributeEntry {
                stat_id,
                param: 0,
                raw_value,
                opaque_bits: None,
            });
        }
    }

    pub fn set_value(&mut self, stat_id: u32, logical_value: i32, save_add: i32) {
        self.set_raw(stat_id, (logical_value + save_add) as u32);
    }

    pub fn actual_value(&self, stat_id: u32, is_alpha: bool) -> Option<i32> {
        self.entries
            .iter()
            .find(|entry| entry.stat_id == stat_id)
            .and_then(|entry| {
                let save_add = char_stat_save_add(stat_id, is_alpha);
                Some(entry.raw_value as i32 - save_add)
            })
    }
}

pub fn char_stat_save_add(stat_id: u32, is_alpha: bool) -> i32 {
    if is_alpha {
        0
    } else {
        match stat_id {
            0 | 1 | 2 | 3 => 32, // Strength, Energy, Dexterity, Vitality usually have +32 in stat_costs
            _ => stat_cost(stat_id).map(|c| c.save_add).unwrap_or(0),
        }
    }
}

fn char_stat_save_bits(stat_id: u32, is_alpha: bool) -> u32 {
    let bits = match stat_id {
        0 | 1 | 2 | 3 | 4 => 10,
        5 => 8,
        6 | 7 | 8 | 9 | 10 | 11 => 21,
        12 => 7,
        13 => 32,
        14 | 15 => 25,
        _ => {
            // Pass-through fallback: if we have it in stat_costs, use that.
            // This is for DLC/expansion stats that might appear.
            stat_cost(stat_id).map(|c| c.save_bits as u32).unwrap_or(0)
        }
    };

    if is_alpha {
        // Special Alpha overrides could go here if reality-check fails
        bits
    } else {
        bits
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSlotClass {
    InventoryLike,
    EquipmentLike,
    StashLike,
    SocketChild,
    Unknown,
}

pub fn classify_item_slot(item: &Item) -> ItemSlotClass {
    if item.mode == 6 || item.location == 6 {
        return ItemSlotClass::SocketChild;
    }
    if item.page >= 4 {
        return ItemSlotClass::StashLike;
    }
    if item.location == 0 {
        return ItemSlotClass::InventoryLike;
    }
    if (1..=3).contains(&item.location) {
        return ItemSlotClass::EquipmentLike;
    }
    ItemSlotClass::Unknown
}

pub fn collect_player_slots(
    bytes: &[u8],
    huffman: &HuffmanTree,
) -> io::Result<Vec<(Item, ItemSlotClass)>> {
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    let alpha_mode = version == 105;
    let items = Item::read_player_items(bytes, huffman, alpha_mode)?;
    let mut slots = Vec::new();
    for item in items {
        push_with_children(&mut slots, item);
    }
    Ok(slots)
}

fn push_with_children(slots: &mut Vec<(Item, ItemSlotClass)>, mut item: Item) {
    let children = mem::take(&mut item.socketed_items);
    let class = classify_item_slot(&item);
    slots.push((item, class));
    for child in children {
        push_with_children(slots, child);
    }
}

pub fn rebuild_status_and_player_items(
    bytes: &[u8],
    attributes: Option<&AttributeSection>,
    skills: Option<&SkillSection>,
    quests: Option<&QuestSection>,
    waypoints: Option<&WaypointSection>,
    expansion: Option<&ExpansionSection>,
    items: &[Item],
    huffman: &HuffmanTree,
) -> io::Result<Vec<u8>> {
    let map = map_core_sections(bytes)?;
    let mut result = Vec::with_capacity(bytes.len() + 64);

    // 1. Prefix: Header up to 'gf' marker
    let mut header_bytes = bytes[..map.gf_pos].to_vec();
    
    // Detect version from header_bytes
    let version = u32::from_le_bytes(header_bytes[4..8].try_into().unwrap_or([0; 4]));

    // Update QUESTS if present (Alpha v105)
    if version == 105 {
        if let Some(qs) = quests {
            let offset = 0x193; // Quest Section (Woo!)
            let slice = qs.as_slice();
            let len = slice.len().min(701 - offset); // End before WS starts at 701
            if header_bytes.len() >= offset + len {
                header_bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }
    }

    // Update WAYPOINTS if present (Alpha v105)
    if version == 105 {
        if let Some(wps) = waypoints {
            let offset = 0x2BD; // Waypoint Section (WS)
            let slice = wps.as_slice();
            let len = slice.len().min(782 - offset); // End before NPC starts at 782
            if header_bytes.len() >= offset + len {
                header_bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }
    }

    // Update NPC Section if present (Alpha v105)
    if version == 105 {
        if let Some(npc) = expansion {
            let offset = 0x30E; // NPC Section
            let slice = npc.as_slice();
            let len = slice.len().min(833 - offset); // Up to header end (Stats start at 833)
            if header_bytes.len() >= offset + len {
                header_bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }
    }
    
    result.extend_from_slice(&header_bytes);

    let is_alpha = version == 105;
    // 2. GF Section
    if let Some(attr) = attributes {
        result.extend_from_slice(&attr.to_bytes(is_alpha)?);
    } else {
        result.extend_from_slice(&bytes[map.gf_pos..map.if_pos]);
    }

    // 3. IF Section (Marker + 30 bytes skills)
    if let Some(skills) = skills {
        result.extend_from_slice(b"if");
        result.extend_from_slice(skills.as_slice());
    } else {
        let skill_end = map.if_pos + 2 + SKILL_SECTION_LEN;
        result.extend_from_slice(&bytes[map.if_pos..skill_end]);
    }

    // 4. Quest/Progression Section (Gap between IF end and first JM)
    // For Alpha v105, quests are in header, no gap section expected.
    let jm0 = map.jm_positions[0];
    let skill_end_original = map.if_pos + 2 + SKILL_SECTION_LEN;
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));

    if version != 105 {
        if let Some(q) = quests {
            result.extend_from_slice(q.as_slice());
        } else if jm0 > skill_end_original {
            result.extend_from_slice(&bytes[skill_end_original..jm0]);
        }
    }

    // 5. Item Sections (Player, Corpse, etc.)
    let jm0 = map.jm_positions[0];
    result.extend_from_slice(&bytes[jm0..]);

    let is_alpha = version == 105;
    rebuild_item_section(&result, items, huffman, is_alpha)
}

pub fn patch_level(bytes: &[u8], new_level: u8, huffman: &HuffmanTree) -> io::Result<Vec<u8>> {
    let map = map_core_sections(bytes)?;
    let mut attrs = AttributeSection::parse(bytes, &map)?;

    // gf 섹션의 level 수정 (stat_id=12, save_add=0, bit_width=7)
    attrs.set_raw(12, new_level as u32);

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    // 헤더 CHAR_LEVEL_OFFSET (27번 바이트) 동기화
    let mut working = rebuild_status_and_player_items(
        bytes,
        Some(&attrs),
        None,
        None,
        None,
        None,
        &Item::read_player_items(bytes, huffman, version == 105)?,
        huffman,
    )?;
    working[CHAR_LEVEL_OFFSET] = new_level;
    finalize_save_bytes(&mut working)?;
    Ok(working)
}


#[derive(Clone)]
pub struct SkillSection([u8; SKILL_SECTION_LEN]);

impl SkillSection {
    pub fn from_slice(slice: &[u8]) -> io::Result<Self> {
        if slice.len() != SKILL_SECTION_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "skill slice does not match expected length",
            ));
        }
        let mut data = [0u8; SKILL_SECTION_LEN];
        data.copy_from_slice(slice);
        Ok(SkillSection(data))
    }

    pub fn as_slice(&self) -> &[u8; SKILL_SECTION_LEN] {
        &self.0
    }
}

pub fn parse_skill_section(bytes: &[u8], map: &SaveSectionMap) -> io::Result<SkillSection> {
    let start = map.if_pos + 2;
    let end = start + SKILL_SECTION_LEN;
    if end > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "skill section truncated",
        ));
    }
    SkillSection::from_slice(&bytes[start..end])
}

pub fn patch_skill_section(
    bytes: &[u8],
    map: &SaveSectionMap,
    skills: &SkillSection,
) -> io::Result<Vec<u8>> {
    let start = map.if_pos + 2;
    let end = start + SKILL_SECTION_LEN;
    if end > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "skill section truncated",
        ));
    }

    let mut rebuilt = bytes.to_vec();
    rebuilt[start..end].copy_from_slice(skills.as_slice());
    finalize_save_bytes(&mut rebuilt)?;
    Ok(rebuilt)
}

#[derive(Debug, Clone)]
pub struct WaypointSection {
    pub raw_bytes: Vec<u8>,
}

impl WaypointSection {
    pub fn from_slice(slice: &[u8]) -> Self {
        WaypointSection {
            raw_bytes: slice.to_vec(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn set_activated(&mut self, byte_idx: usize, bit_idx: usize, active: bool) {
        if byte_idx < self.raw_bytes.len() {
            if active {
                self.raw_bytes[byte_idx] |= 1 << bit_idx;
            } else {
                self.raw_bytes[byte_idx] &= !(1 << bit_idx);
            }
        }
    }

    pub fn is_activated_by_name(&self, name: &str, difficulty: u8) -> bool {
        if let Some(entry) = crate::data::waypoints::WAYPOINTS.iter().find(|e| e.name == name) {
            // WS Section Layout:
            // 0..8: "WS" Header
            // 8..32: Normal (8..10: 02 01, 10..32: Data)
            // 32..56: Nightmare (32..34: 02 01, 34..56: Data)
            // 56..80: Hell (56..58: 02 01, 58..80: Data)
            let global_bit_idx = (8 * 8) + (difficulty as usize * 24 * 8) + (2 * 8) + entry.ws_bit as usize;
            let byte_idx = global_bit_idx / 8;
            let bit_in_byte = global_bit_idx % 8;
            if byte_idx < self.raw_bytes.len() {
                return self.raw_bytes[byte_idx] & (1 << bit_in_byte) != 0;
            }
        }
        false
    }

    pub fn set_activated_by_name(&mut self, name: &str, difficulty: u8, active: bool) -> bool {
        if let Some(entry) = crate::data::waypoints::WAYPOINTS.iter().find(|e| e.name == name) {
            let global_bit_idx = (8 * 8) + (difficulty as usize * 24 * 8) + (2 * 8) + entry.ws_bit as usize;
            let byte_idx = global_bit_idx / 8;
            let bit_in_byte = global_bit_idx % 8;
            self.set_activated(byte_idx, bit_in_byte, active);
            return true;
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct ExpansionSection {
    pub raw_bytes: Vec<u8>,
}

impl ExpansionSection {
    pub fn from_slice(slice: &[u8]) -> Self {
        ExpansionSection {
            raw_bytes: slice.to_vec(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn is_activated_by_name(&self, difficulty: u8, name: &str) -> bool {
        if let Some(entry) = crate::data::waypoints::WAYPOINTS.iter().find(|e| e.name == name) {
            // ExpansionSection implementation for waypoints (fallback/legacy)
            // It assumes raw_bytes starts at 'WS' marker (0x2BD) or follows standard WS mapping.
            let global_bit_idx = (difficulty as usize * 24 * 8) + (10 * 8) + entry.ws_bit as usize;
            let byte_idx = global_bit_idx / 8;
            let bit_in_byte = global_bit_idx % 8;
            if byte_idx < self.raw_bytes.len() {
                return self.raw_bytes[byte_idx] & (1 << bit_in_byte) != 0;
            }
        }
        false
    }

    pub fn set_activated_by_name(&mut self, name: &str, difficulty: u8, active: bool) -> bool {
        if let Some(entry) = crate::data::waypoints::WAYPOINTS.iter().find(|e| e.name == name) {
            let global_bit_idx = (difficulty as usize * 24 * 8) + (10 * 8) + entry.ws_bit as usize;
            let byte_idx = global_bit_idx / 8;
            let bit_in_byte = global_bit_idx % 8;
            if byte_idx < self.raw_bytes.len() {
                if active {
                    self.raw_bytes[byte_idx] |= 1 << bit_in_byte;
                } else {
                    self.raw_bytes[byte_idx] &= !(1 << bit_in_byte);
                }
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone)]
pub struct QuestSection {
    pub raw_bytes: Vec<u8>,
}

impl QuestSection {
    pub fn from_slice(slice: &[u8]) -> Self {
        QuestSection {
            raw_bytes: slice.to_vec(),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.raw_bytes
    }

    pub fn is_v105_completed_by_name(&self, name: &str) -> bool {
        if let Some(entry) = crate::data::quests::V105_QUESTS.iter().find(|e| e.name == name) {
            let offset = entry.v105_offset - 403; // 0x193 relative
            if offset < self.raw_bytes.len() {
                let val = self.raw_bytes[offset];
                // In v105, 0x01 is the completion bit (bit 0)
                return (val & 0x01) != 0;
            }
        }
        false
    }

    pub fn set_v105_completed_by_name(&mut self, name: &str, completed: bool) -> bool {
        if let Some(entry) = crate::data::quests::V105_QUESTS.iter().find(|e| e.name == name) {
            let offset = entry.v105_offset - 403; // 0x193 relative
            if offset + 1 < self.raw_bytes.len() {
                if completed {
                    // Set Byte 0 bit 0 (Completed)
                    self.raw_bytes[offset] |= 0x01;
                    // Set Byte 1 bit 4 (Checked/Seen - 0x10)
                    self.raw_bytes[offset + 1] |= 0x10;
                } else {
                    self.raw_bytes[offset] &= !0x01;
                    self.raw_bytes[offset + 1] &= !0x10;
                }
                return true;
            }
        }
        false
    }

    /// Unlocks the Durance of Hate gate (Act 3) by setting semantic bits discovered in forensics.
    pub fn unlock_durance_gate(&mut self) {
        // 1. Set "Khalim's Will" Quest Completed Bits
        self.set_v105_completed_by_name("Khalim's Will", true);
        
        // 2. Set "Sacred Authority" / Gate Flag in the Quest Section Header (approx byte 8)
        if self.raw_bytes.len() > 8 {
            self.raw_bytes[8] |= 0x01; // Gate Flag
        }
        
        // 3. Set Environment State (approx 12th byte / before first quest)
        if self.raw_bytes.len() > 11 {
            self.raw_bytes[11] |= 0x80; // Orb Destroyed / Environment Trigger
        }
    }
}

pub fn parse_quest_section(bytes: &[u8], map: &SaveSectionMap) -> io::Result<QuestSection> {
    let skill_end = map.if_pos + 2 + SKILL_SECTION_LEN;
    let jm0 = map.jm_positions[0];
    if jm0 < skill_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "JM section starts before skill section ends",
        ));
    }
    Ok(QuestSection::from_slice(&bytes[skill_end..jm0]))
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
            raw_prefix: bytes[..match version {
                105 => 833, // Alpha v105 Fixed Header
                _ => MIN_HEADER_LEN,
            }
            .min(bytes.len())]
                .to_vec(),
            quests: if version == 105 && bytes.len() >= 0x193 + 12 {
                // Quest Section (Woo!) starts at 0x193 (403).
                // It ends before Waypoints at 0x2BD (701).
                let end = 0x2BD.min(bytes.len());
                Some(QuestSection::from_slice(&bytes[0x193..end]))
            } else {
                None
            },
            waypoints: if version == 105 && bytes.len() >= 0x2BD + 2 {
                // Waypoint Section (WS) starts at 0x2BD (701).
                // It spans 24 bytes per difficulty (72 bytes total).
                // NPC section starts at 0x30E (782).
                let end = 0x30E.min(bytes.len());
                Some(WaypointSection::from_slice(&bytes[0x2BD..end]))
            } else {
                None
            },
            expansion: if version == 105 && bytes.len() >= 0x30E + 2 {
                // NPC Section starts at 0x30E (782).
                // It ends at header end (833).
                let end = 833.min(bytes.len());
                Some(ExpansionSection::from_slice(&bytes[0x30E..end]))
            } else {
                None
            },
        };

        Ok(Save { header })
    }
}

impl Header {
    pub fn to_bytes(&self) -> io::Result<Vec<u8>> {
        let mut bytes = self.raw_prefix.clone();

        // Update QUESTS if present (Alpha v105)
        if let Some(ref qs) = self.quests {
            let offset = 0x193;
            let slice = qs.as_slice();
            let max_len = 0x2BD - offset;
            let len = slice.len().min(max_len);
            if bytes.len() >= offset + len {
                bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }

        // Update WAYPOINTS if present (Alpha v105)
        if let Some(ref wps) = self.waypoints {
            let offset = 0x2BD;
            let slice = wps.as_slice();
            let max_len = 0x30E - offset;
            let len = slice.len().min(max_len);
            if bytes.len() >= offset + len {
                bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }

        // Update EXPANSION (NPC) if present (Alpha v105)
        if let Some(ref ex) = self.expansion {
            let offset = 0x30E;
            let slice = ex.as_slice();
            let max_len = 833 - offset;
            let len = slice.len().min(max_len);
            if bytes.len() >= offset + len {
                bytes[offset..offset + len].copy_from_slice(&slice[..len]);
            }
        }

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
        let header_bytes = self.header.to_bytes()?;
        let header_len = header_bytes.len();
        if bytes.len() < header_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Target buffer is too short to receive header bytes.",
            ));
        }
        bytes[..header_len].copy_from_slice(&header_bytes);
        finalize_save_bytes(bytes)?;
        Ok(())
    }
}

pub fn rebuild_item_section(
    bytes: &[u8],
    items: &[Item],
    huffman: &HuffmanTree,
    alpha_mode: bool,
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

    let mut serialized_section = Item::serialize_section(items, huffman, alpha_mode)?;

    if items.len() > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "item count exceeds 65535",
        ));
    }
    let count_u16 = items.len() as u16;

    let section_start = jm1 + 4;
    let original_section_len = jm2 - section_start;
    let _original_section = &bytes[section_start..jm2];
    let serialized_len_before_padding = serialized_section.len();
    if serialized_len_before_padding < original_section_len {
        let missing_start = section_start + serialized_len_before_padding;
        serialized_section.extend_from_slice(&bytes[missing_start..jm2]);
    }
    // Research Hack: Allow serialized section to differ from original.
    /*
    if serialized_section != original_section {
        let mut fallback = bytes.to_vec();
        finalize_save_bytes(&mut fallback)?;
        return Ok(fallback);
    }
    */
    let final_section_len = serialized_section.len();
    let mut rebuilt =
        Vec::with_capacity(bytes.len() - original_section_len + final_section_len);
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
        let prefix_len = save.header.raw_prefix.len();
        assert_eq!(header_bytes, bytes[..prefix_len]);
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
