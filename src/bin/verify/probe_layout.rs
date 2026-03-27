use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    let start_bit: u64 = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(7790);
    let target_list2: u64 = 7962;

    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();

    println!(
        "--- Alpha v105 List 1 Combinatorial Probe (Start={}, TargetL2={}) ---",
        start_bit, target_list2
    );

    // Try various N-bit ID + M-bit Val combinations
    for id_bits in 8..=11 {
        for val_bits in 1..=32 {
            let entry_width = id_bits + val_bits;
            for num_stats in 1..=10 {
                let total_bits = (num_stats as u64) * (entry_width as u64) + (id_bits as u64); // Including terminator
                if start_bit + total_bits == target_list2 {
                    println!(
                        "MATCH FOUND: Entry={} bits (ID={} + VAL={}), Stats={}",
                        entry_width, id_bits, val_bits, num_stats
                    );
                }
            }
        }
    }
}
