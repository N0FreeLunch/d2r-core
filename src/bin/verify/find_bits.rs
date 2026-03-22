use d2r_core::save::Section;
use std::fs;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let sections = Section::from_bytes(&bytes).unwrap();
    
    for section in sections {
        if section.code == "JM" {
            println!("JM Section at 0x{:04X}: {} items", section.offset, section.items_count);
            let mut current_bit = (section.offset as u64 + 4) * 8;
            for i in 0..section.items_count {
                // For Alpha v105, we know potions have 76 bits.
                // We'll just assume a fixed width for now to see where index 5 is.
                println!("  Item {} start bit: {}", i, current_bit);
                current_bit += 76; // Potion width estimate
            }
        }
    }
}
