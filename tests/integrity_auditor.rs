use std::fs;

#[test]
fn test_serialization_literal_bit_integrity() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let file_path = format!("{}/src/domain/item/serialization.rs", manifest_dir);
    let content = fs::read_to_string(&file_path)
        .expect("Failed to read src/domain/item/serialization.rs");

    // Rule 1: Retail/Runeword variable-width property emission
    let rule_1 = content.contains("emitter.write_bits(prop.raw_value as u32, stat.save_bits as u32)?;");
    assert!(rule_1, "Integrity Violation: Retail-width property emission (stat.save_bits) drifted");

    // Rule 2: Property terminator must be 9 bits
    let rule_2_val = content.contains("let id_bits = 9;");
    let rule_2_call = content.contains("emitter.write_bits(terminator, id_bits)?;");
    assert!(rule_2_val && rule_2_call, "Integrity Violation: 9-bit property list terminator drifted");

    // Rule 3: Alpha tail alignment (1-bit 0 + byte align)
    let rule_3_comment = content.contains("Alpha v105 Requirement: Mandatory 1 Terminal Bit (0) + Padding to Byte Boundary");
    let rule_3_seq = content.contains("emitter.write_bit(false)?;") && content.contains("emitter.byte_align()?;");
    assert!(rule_3_comment && rule_3_seq, "Integrity Violation: Alpha tail alignment pattern Drifted");
}
