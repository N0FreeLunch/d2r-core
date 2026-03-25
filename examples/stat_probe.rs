use d2r_core::save::{map_core_sections, gf_payload_range};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = "tests/fixtures/savegames/original/amazon_10_scrolls.d2s";
    let bytes = fs::read(path)?;
    let map = map_core_sections(&bytes)?;
    let payload_range = gf_payload_range(&map);
    
    println!("Probing Stat Section in {} (0x{:04X}..0x{:04X})", path, payload_range.start, payload_range.end);
    
    for bits in [9, 10] {
        println!("\n--- Attempting {}-bit Stat ID Read ---", bits);
        let mut reader = BitReader::endian(
            Cursor::new(&bytes[payload_range.start..payload_range.end]),
            LittleEndian,
        );
        let total_bits = ((payload_range.end - payload_range.start) * 8) as u64;
        let mut count = 0;
        let terminator = (1 << bits) - 1;
        
        loop {
            let pos = reader.position_in_bits()?;
            if total_bits.saturating_sub(pos) < bits as u64 {
                println!("  [END] Reached end of buffer without terminator.");
                break;
            }
            
            let stat_id = match bits {
                9 => reader.read::<9, u32>()?,
                10 => reader.read::<10, u32>()?,
                _ => unreachable!(),
            };
            
            if stat_id == terminator {
                println!("  [OK] Found Terminator (0x{:X}) at bit {} after {} stats.", terminator, pos, count);
                break;
            }
            
            // For probing, we skip the value bits.
            // Problem: we don't know the exact value bits unless we have the correct ID width.
            // But we can guess from the current char_stat_save_bits logic.
            let val_bits = match stat_id {
                0..=4 => 10,
                5 => 8,
                6..=11 => 21,
                12 => 7,
                13 => 32,
                14..=15 => 25,
                _ => {
                    println!("  [FAIL] Unknown Stat ID {} at bit {}. Desync likely.", stat_id, pos);
                    break;
                }
            };
            
            if reader.skip(val_bits).is_err() {
                println!("  [FAIL] Failed to skip {} bits at bit {}.", val_bits, pos);
                break;
            }
            count += 1;
            if count > 100 { break; } // Safety
        }
    }
    
    Ok(())
}

fn stat_cost_bits_guess(id: u32) -> u32 {
    match id {
        0..=4 => 10,
        5 => 8,
        6..=11 => 21,
        12 => 7,
        13 => 32,
        14..=15 => 25,
        _ => 0,
    }
}
