use std::fs;

fn main() {
    let bytes = fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let targets = [310, 14, 31, 1];
    
    println!("--- Searching for Specific Target Values (v2) ---");

    for bit in 7700..8100 {
        for width in 1..=12 {
            let mut val = 0u32;
            for i in 0..width {
                let bit_idx = (bit + i as u64) / 8;
                let bit_off = (bit + i as u64) % 8;
                if (bytes[bit_idx as usize] >> bit_off) & 1 == 1 {
                    val |= 1 << i;
                }
            }
            if targets.contains(&val) {
                // Potential target found. Now check for neighbors.
                for bit2 in bit-64..bit+64 {
                    if bit2 == bit { continue; }
                    for width2 in 1..=12 {
                        let mut val2 = 0u32;
                        for j in 0..width2 {
                            let bit2_idx = (bit2 + j as u64) / 8;
                            let bit2_off = (bit2 + j as u64) % 8;
                            if (bytes[bit2_idx as usize] >> bit2_off) & 1 == 1 {
                                val2 |= 1 << j;
                            }
                        }
                        if targets.contains(&val2) && val2 != val {
                             println!("FOUND CLUSTER! Bit:{} Val:{}, Near Bit:{} Val:{}", bit, val, bit2, val2);
                        }
                    }
                }
            }
        }
    }
}
