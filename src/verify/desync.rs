use serde::Serialize;
use crate::item::{Item, HuffmanTree, peek_item_header_at, is_plausible_item_header};
use crate::error::ParsingResult;

#[derive(Debug, Clone, Serialize)]
pub struct DesyncReport {
    pub item_index: usize,
    pub oracle_start: u64,
    pub parser_start: u64,
    pub drift: i64,
    pub item_code: String,
    pub is_match: bool,
}

/// Detects bit-level drift between empirical item starts (Oracle) and parser-calculated starts.
pub fn detect_desync(bytes: &[u8], huffman: &HuffmanTree, is_alpha: bool) -> ParsingResult<Vec<DesyncReport>> {
    // 1. Find the JM section
    let jm_pos = (0..bytes.len().saturating_sub(1))
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .ok_or_else(|| crate::error::ParsingFailure {
            error: crate::error::ParsingError::MissingMarker { marker: "JM".to_string(), bit_offset: 0 },
            context_stack: vec!["detect_desync".to_string()],
            bit_offset: 0,
            context_relative_offset: 0,
            hint: Some("No JM marker found".to_string()),
        })?;

    let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
    let section_payload = &bytes[jm_pos + 4..];
    let section_base_bit = ((jm_pos + 4) * 8) as u64;

    // 2. Standard Parsing (Try to get as many as possible even on failure)
    let parsed_items = match Item::read_section(section_payload, count, huffman, is_alpha) {
        Ok(items) => items,
        Err(_) => {
            // If it failed, we can't easily get the partial items from read_section without refactoring it.
            // For now, let's assume we want to catch drift in successfully parsed (but potentially wrong) item lists.
            // Or we could implement a more granular loop here.
            Vec::new() 
        }
    };
    
    if parsed_items.is_empty() && count > 0 {
         // Try a more granular parse to catch at least the first few items
         // (Implementation detail: if read_section failed, we might need a custom loop here)
    }

    // 3. Oracle Search
    let mut oracle_starts = Vec::new();
    let mut bit_idx = section_base_bit;
    let total_bits = (bytes.len() * 8) as u64;
    
    let mut found_count = 0;
    while bit_idx < total_bits.saturating_sub(100) && found_count < count as usize {
        if let Some((mode, location, _x, code, flags, version, _is_compact, _header_bits, _nudge)) = 
            peek_item_header_at(bytes, bit_idx, huffman, is_alpha) 
        {
            if is_plausible_item_header(mode, location, &code, flags, version, is_alpha) {
                if !is_alpha || version == 5 {
                    oracle_starts.push((bit_idx, code.trim().to_string()));
                    found_count += 1;
                    bit_idx += 72; // Jump minimum item size
                    continue;
                }
            }
        }
        bit_idx += 8;
    }

    // 4. Comparison
    let mut reports = Vec::new();
    let compare_count = parsed_items.len().min(oracle_starts.len());

    for i in 0..compare_count {
        let oracle_start = oracle_starts[i].0;
        let parser_start = section_base_bit + parsed_items[i].range.start;
        let drift = parser_start as i64 - oracle_start as i64;
        
        reports.push(DesyncReport {
            item_index: i,
            oracle_start,
            parser_start,
            drift,
            item_code: oracle_starts[i].1.clone(),
            is_match: drift == 0,
        });
    }

    Ok(reports)
}
