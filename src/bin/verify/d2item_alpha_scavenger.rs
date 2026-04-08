use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::{HuffmanTree, is_plausible_item_header};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2item_alpha_scavenger <save_file>");
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).expect("failed to read save file");

    let huffman = HuffmanTree::new();
    let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];
    if !is_alpha {
        println!("Not an Alpha v105 save file.");
        return;
    }

    let jm_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .expect("No JM marker found");
    
    let start_bit = (jm_pos + 4) * 8;
    let end_bit = bytes.len() * 8;

    println!("Scavenging for Alpha items from bit {} to {}:", start_bit, end_bit);

    for bit in start_bit as u64..end_bit as u64 {
        let reader = IoBitReader::endian(Cursor::new(&bytes[(bit / 8) as usize..]), LittleEndian);
        let mut cursor = BitCursor::new(reader);
        let _ = cursor.skip_and_record((bit % 8) as u32);

        let checkpoint = cursor.pos();
        let flags = match cursor.read_bits::<u32>(32) { Ok(v) => v, _ => continue };
        let version = match cursor.read_bits::<u32>(3) { Ok(v) => v, _ => continue };
        let mode = match cursor.read_bits::<u32>(3) { Ok(v) => v, _ => continue };
        let loc = match cursor.read_bits::<u32>(3) { Ok(v) => v, _ => continue };
        let x = match cursor.read_bits::<u32>(4) { Ok(v) => v, _ => continue };
        
        let header_bits_before_gap = cursor.pos() - checkpoint;

        for gap in 0..=16u64 {
            let total_offset = bit + header_bits_before_gap + gap;
            let g_reader = IoBitReader::endian(Cursor::new(&bytes[(total_offset / 8) as usize..]), LittleEndian);
            let mut g_cursor = BitCursor::new(g_reader);
            let _ = g_cursor.skip_and_record((total_offset % 8) as u32);

            let mut code = String::new();
            let mut fail = false;
            for _ in 0..4 {
                match huffman.decode_recorded(&mut g_cursor) {
                    Ok(ch) => code.push(ch),
                    _ => { fail = true; break; }
                }
            }

            let axiom = d2r_core::domain::header::entity::HeaderAxiom {
                version: version as u8,
                alpha_mode: true,
            };

            if !fail && axiom.is_plausible(mode as u8, loc as u8, &code, flags) {
                println!("  [Bit {}] Potential Item: '{}' (mode={}, loc={}, x={}, flags=0x{:08X}, gap={})", 
                    bit, code, mode, loc, x, flags, gap);
            }
        }
    }
}
