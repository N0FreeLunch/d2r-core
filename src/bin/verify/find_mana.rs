use std::fs;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let val = 14u32;
    
    println!("--- Searching for Value {} (4-9 bits) ---", val);

    for width in 4..=9 {
        let pattern: Vec<u8> = (0..width).map(|i| ((val >> i) & 1) as u8).collect();
        for bit in 7744..8500 {
            let mut matches = true;
            for (i, &p) in pattern.iter().enumerate() {
                let bit_idx = (bit + i as u64) / 8;
                let bit_off = (bit + i as u64) % 8;
                if bit_idx as usize >= bytes.len() { matches = false; break; }
                let b = (bytes[bit_idx as usize] >> bit_off) & 1;
                if b != p as u8 {
                    matches = false;
                    break;
                }
            }
            if matches {
                println!("  FOUND {} at bit {} (Width {})", val, bit, width);
            }
        }
    }
}
