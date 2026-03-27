use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::{self, Cursor};

/// Forensic Structural Anchor Scanner for Alpha v105
/// Promoted from experimental jm_scan_v9.
/// Scans for JM markers and validates Alpha-specific structural anchors.
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 4 {
        println!("Usage: d2item_forensic_scan <save_path> <start_bit> <end_bit>");
        return Ok(());
    }

    let path = &args[1];
    let start_bit: u64 = args[2].parse().unwrap();
    let end_bit: u64 = args[3].parse().unwrap();

    let bytes = fs::read(path)?;
    println!(
        "[ForensicScan] File: {} | Range: {}..{}",
        path, start_bit, end_bit
    );

    for bit_pos in (start_bit..=end_bit).step_by(8) {
        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let _ = reader.skip(bit_pos as u32);

        let m1 = reader.read::<8, u8>().unwrap_or(0);
        let m2 = reader.read::<8, u8>().unwrap_or(0);

        if m1 == 0x4A && m2 == 0x4D {
            // 'JM'
            // Header: Flags(32), Ver(3), Mode(3), Loc(4), X(4), Y(4/0), Page(3/0)...
            let flags = reader.read::<32, u32>().unwrap_or(0);
            let version = reader.read_var::<u8>(3).unwrap_or(0);
            let mode = reader.read_var::<u8>(3).unwrap_or(0);
            let loc = reader.read_var::<u8>(4).unwrap_or(0);

            let is_alpha = version == 5 || version == 1 || version == 0;

            if is_alpha {
                // In Alpha, we expect an 8-bit Gap(0x00) for storage items (loc < 4)
                // The gap is after the base header (flags, ver, mode, loc, x, y, page, socket_hint)
                // Base header bits: 32 + 3 + 3 + 4 + 4 + 4 + 3 + 3 = 56 bits
                // Plus marker (16) = 72 bits
                let mut gap_reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
                let _ = gap_reader.skip((bit_pos + 72) as u32);
                let gap = gap_reader.read::<8, u8>().unwrap_or(255);

                print!(
                    "  [Match] at {}: ver={}, mode={}, loc={}, gap={:#04x}",
                    bit_pos, version, mode, loc, gap
                );

                if gap == 0x00 && loc < 4 {
                    println!("  (VALID ALPHA ANCHOR)");
                } else if loc >= 4 {
                    println!("  (Equipped/Belt/etc)");
                } else {
                    println!("  (Suspect: Missing Gap)");
                }
            } else {
                println!(
                    "  [Match] at {}: ver={}, mode={}, loc={} (Modern)",
                    bit_pos, version, mode, loc
                );
            }
        }
    }

    Ok(())
}
