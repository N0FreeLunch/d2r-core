use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn print_item_detail(item: &Item, index: usize, bit_start: u64, bit_end: u64) {
    let mut s = String::new();
    for &b in &item.bits {
        s.push(if b { '1' } else { '0' });
    }

    let mut flags = 0u32;
    for (i, &b) in item.bits.iter().enumerate().take(32) {
        if b {
            flags |= 1 << i;
        }
    }

    println!("--- Item {} ---", index);
    println!("  Code:           '{}'", item.code);
    println!("  Bit start:      {} (byte {})", bit_start, bit_start / 8);
    println!("  Bit end:        {} (byte {})", bit_end, bit_end / 8);
    println!("  Bit length:     {}", bit_end - bit_start);
    println!("  Flags:          0x{:08X}", flags);
    println!("  Is Identified:  {}", (flags & 0x0010) != 0);
    println!("  Is Compact:     {}", (flags & 0x0020_0000) != 0);
    println!("  Bits:           {}", s);
}

fn inspect_d2i(path: &str) {
    println!("=== Inspecting .d2i: {} ===", path);
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  [ERROR] {}", e);
            return;
        }
    };

    let huffman = HuffmanTree::new();
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    let bit_start = 0u64;
    match Item::from_reader(&mut reader, &huffman) {
        Ok(item) => {
            let bit_end = reader.position_in_bits().unwrap_or(0);
            print_item_detail(&item, 0, bit_start, bit_end);
        }
        Err(e) => eprintln!("  [ERROR] Failed to parse item: {}", e),
    }
}

fn inspect_d2s(path: &str, filter_code: Option<&str>) {
    println!("=== Inspecting .d2s: {} ===", path);
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("  [ERROR] {}", e);
            return;
        }
    };

    // Find first JM
    let jm_pos =
        (0..bytes.len().saturating_sub(1)).find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M');

    let jm = match jm_pos {
        Some(p) => p,
        None => {
            eprintln!("  [ERROR] No JM marker found");
            return;
        }
    };

    let item_count = u16::from_le_bytes([bytes[jm + 2], bytes[jm + 3]]);
    println!("  First JM at byte {jm}, item count: {item_count}");

    let huffman = HuffmanTree::new();
    let mut reader = BitReader::endian(Cursor::new(&bytes[jm..]), LittleEndian);

    // Skip JM + count (4 bytes = 32 bits)
    let _: u32 = reader.read::<32, u32>().unwrap_or(0);

    for i in 0..item_count {
        let bit_start = (jm * 8) as u64 + reader.position_in_bits().unwrap_or(0);
        match Item::from_reader(&mut reader, &huffman) {
            Ok(item) => {
                let bit_end = (jm * 8) as u64 + reader.position_in_bits().unwrap_or(0);
                if let Some(code) = filter_code {
                    if item.code.trim() != code.trim() {
                        continue;
                    }
                }
                print_item_detail(&item, i as usize, bit_start, bit_end);
            }
            Err(e) => {
                eprintln!("  [ERROR] at item {}: {}", i, e);
                break;
            }
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2item_inspect <file.d2s|file.d2i> [--code <code>]");
        eprintln!("  --code   Filter to only show items with this code (e.g. 'tsc')");
        process::exit(1);
    }

    let path = &args[1];
    let filter_code = args
        .windows(2)
        .find(|w| w[0] == "--code")
        .map(|w| w[1].as_str());

    if path.ends_with(".d2i") {
        inspect_d2i(path);
    } else if path.ends_with(".d2s") {
        inspect_d2s(path, filter_code);
    } else {
        eprintln!("[ERROR] Unknown file type. Expected .d2s or .d2i");
        process::exit(1);
    }
}
