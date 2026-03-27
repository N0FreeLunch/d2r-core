use std::env;
use std::fs;

fn main() {
    let path = "tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s";
    let bytes = fs::read(path).expect("Failed to read file");
    let gf_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'g' && bytes[i + 1] == b'f')
        .expect("gf not found");

    println!("gf at 0x{:X}", gf_pos);
    let payload = &bytes[gf_pos + 2..];

    let mut bits = Vec::new();
    for byte in payload {
        for bit_idx in 0..8 {
            bits.push((byte >> bit_idx) & 1 != 0);
        }
    }

    // Dump bit stream
    println!("Bit stream (first 256 bits):");
    for i in 0..256 {
        if i % 9 == 0 {
            print!("(");
        }
        print!("{}", if bits[i] { '1' } else { '0' });
        if i % 9 == 8 {
            print!(") ");
        }
        if i % 72 == 71 {
            println!();
        }
    }
    println!();
}
