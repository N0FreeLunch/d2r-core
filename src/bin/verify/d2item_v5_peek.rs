use std::fs;
use std::io::{self, Cursor};
use bitstream_io::{BitRead, BitReader, LittleEndian};

/// Enhanced Bit Peeker for Alpha v105 Items
/// Promoted from experimental forensic scripts.
/// Decodes item headers and structural gaps at the bit-level.
fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: d2item_v5_peek <save_path> <start_bit> [dump_len]");
        return Ok(());
    }

    let path = &args[1];
    let start_bit: u64 = args[2].parse().unwrap();
    let dump_len: u32 = if args.len() > 3 { args[3].parse().unwrap() } else { 128 };

    let bytes = fs::read(path)?;
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    let _ = reader.skip(start_bit as u32);

    println!("[V5Peek] File: {} | Offset: {}", path, start_bit);
    
    // Header Decoding (Direct flags at offset)
    let flags = reader.read::<32, u32>().unwrap_or(0);
    println!("Flags: {:#010x}", flags);
    
    let ver = reader.read_var::<u8>(3).unwrap_or(0);
    let mode = reader.read_var::<u8>(3).unwrap_or(0);
    let loc = reader.read_var::<u8>(4).unwrap_or(0);
    println!("Ver: {}, Mode: {}, Loc: {}", ver, mode, loc);
    
    let x = reader.read_var::<u8>(4).unwrap_or(0);
    let y = reader.read_var::<u8>(4).unwrap_or(0);
    println!("X: {}, Y: {}", x, y);
    
    let page = reader.read_var::<u8>(3).unwrap_or(0);
    let hint = reader.read_var::<u8>(3).unwrap_or(0);
    println!("Page: {}, SocketHint: {}", page, hint);
    
    if ver == 5 || ver == 1 || ver == 0 {
        let gap = reader.read::<8, u8>().unwrap_or(255);
        println!("Alpha Gap (8bit): {:#04x}", gap);
    }
    
    println!("--- Raw Bit Dump ({} bits starts from bit {}) ---", dump_len, start_bit + 72 + 8);
    for i in 0..dump_len {
        let b = reader.read_bit().unwrap_or(false);
        print!("{}", if b { "1" } else { "0" });
        if (i + 1) % 8 == 0 { print!(" "); }
        if (i + 1) % 32 == 0 { println!(); }
    }
    println!();

    Ok(())
}
