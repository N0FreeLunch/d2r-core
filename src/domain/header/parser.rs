use bitstream_io::BitRead;
use std::io;
use crate::data::bit_cursor::BitCursor;
use crate::domain::header::entity::{ItemHeader, HeaderAxiom};

/// Parses an item header from the bit cursor.
pub fn parse_header<R: BitRead>(
    cursor: &mut BitCursor<R>,
    axiom: &HeaderAxiom,
) -> io::Result<ItemHeader> {
    cursor.begin_segment("Header");

    let flags: u32 = cursor.read_bits(32)?;
    let version: u8 = cursor.read_bits(3)?;
    
    let _mode: u8 = cursor.read_bits(3)?;
    let _location: u8 = cursor.read_bits(3)?;
    let _x: u8 = cursor.read_bits(4)?;

    // Handle Alpha v105 detection and sync if needed.
    // For Slice S2, we implement the base parser.
    // Full sync logic from Item::read_item will be migrated later.

    let is_alpha = axiom.alpha_mode && (version == 5 || version == 1);
    
    let is_identified = (flags & (1 << 4)) != 0;
    let is_personalized = if is_alpha { (flags & (1 << 28)) != 0 } else { (flags & (1 << 24)) != 0 };
    let is_runeword = (flags & (1 << 26)) != 0;
    let is_compact = (flags & (1 << 21)) != 0;
    let is_socketed = if is_alpha { (flags & (1 << 27)) != 0 } else { (flags & (1 << 11)) != 0 };
    let is_ethereal = (flags & (1 << 22)) != 0;

    // We don't read Y, Page, etc. here as they are part of the Body in our refined model,
    // or they might be skipped depending on is_compact.
    
    cursor.end_segment();

    Ok(ItemHeader {
        id: None, // Will be read in extended stats
        quality: None, // Will be read in extended stats
        version,
        is_compact,
        is_identified,
        is_socketed,
        is_personalized,
        is_runeword,
        is_ethereal,
    })
}
