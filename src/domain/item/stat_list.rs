use bitstream_io::{BitRead};
use crate::data::bit_cursor::BitCursor;
use crate::item::{HuffmanTree, ParsingResult, PropertyReaderContext};
pub use crate::domain::stats::{ItemProperty, PropertyParseResult};

pub use crate::domain::stats::parser::recover_alpha_xrs_properties;

pub fn read_property_list<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> ParsingResult<(Vec<ItemProperty>, bool, bool)> {
    crate::domain::stats::parser::read_property_list(
        recorder,
        code,
        version,
        section_recovery,
        huffman,
        alpha_runeword,
        crate::item::recover_property_reader,
    )
}

pub fn parse_single_property<R: BitRead>(
    recorder: &mut BitCursor<R>,
    code: &str,
    version: u8,
    section_recovery: PropertyReaderContext,
    huffman: &HuffmanTree,
    alpha_runeword: bool,
) -> ParsingResult<PropertyParseResult> {
    crate::domain::stats::parser::parse_single_property(
        recorder,
        code,
        version,
        section_recovery,
        huffman,
        alpha_runeword,
        crate::item::recover_property_reader,
    )
}
