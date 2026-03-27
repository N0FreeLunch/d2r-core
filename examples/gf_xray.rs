use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::save::map_core_sections;
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes = fs::read("tmp/amazon_gf_mf.d2s").unwrap();
    let map = map_core_sections(&bytes).unwrap();
    let section_bytes = &bytes[map.gf_pos + 2..map.if_pos];

    let mut reader = BitReader::endian(Cursor::new(section_bytes), LittleEndian);
    let total_bits = section_bytes.len() * 8;

    println!("=== GF X-ray (fuzzed) ===");
    let mut pos = 0;
    while pos + 9 <= total_bits {
        let mut stat_id: u32 = 0;
        for i in 0..9 {
            if reader.read_bit().unwrap() {
                stat_id |= 1 << i;
            }
        }
        println!("Bit {}: ID {}", pos, stat_id);
        pos += 9;

        if stat_id == 0x1FF {
            println!("Terminator reached.");
            break;
        }

        let bits = match stat_id {
            0..=4 => 10,
            5 => 8,
            6..=11 => 21,
            12 => 7,
            13 => 32,
            14..=15 => 25,
            _ => 10, // Research-mode guess (e.g. for MF/GF)
        };

        if pos + bits > total_bits {
            println!("Value exceeds range! (bits: {} at pos {})", bits, pos);
            break;
        }

        let mut val: u32 = 0;
        for i in 0..bits {
            if reader.read_bit().unwrap() {
                val |= 1 << i;
            }
        }
        println!("Value: {} ({} bits)", val, bits);
        pos += bits;
    }
}
