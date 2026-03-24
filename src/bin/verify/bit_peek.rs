use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item, BitRecorder};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2item_bit_peek <save_file>");
        return;
    }
    let path = &args[1];
    let offset = args.get(2).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0);
    let count_bits = args.get(3).and_then(|s| s.parse::<u32>().ok()).unwrap_or(64);
    let bytes = fs::read(path).expect("failed to read save file");

    if offset > 0 {
        let mut reader = IoBitReader::endian(Cursor::new(&bytes[(offset/8) as usize..]), LittleEndian);
        let mut recorder = BitRecorder::new(&mut reader);
        for _ in 0..(offset % 8) { recorder.read_bit().unwrap(); }
        let val = recorder.read_bits_u64(count_bits).unwrap_or(0);
        println!("Bits at offset {}: {:0width$b}", offset, val, width=count_bits as usize);
        return;
    }

    let jm_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .expect("No JM marker found");
    let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
    println!("JM at byte {}, item count {}", jm_pos, count);

    let huffman = HuffmanTree::new();
    let mut reader = IoBitReader::endian(Cursor::new(&bytes[jm_pos + 4..]), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);

    let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];
    for i in 0..count {
        let bit_start = (jm_pos + 4) * 8 + recorder.total_read as usize;
        match Item::from_reader_with_context(&mut recorder, &huffman, Some((&bytes, ((jm_pos+4)*8) as u64)), is_alpha) {
            Ok(item) => {
                println!(
                    "Item {}: '{}' (start_bit={}, len={} bits)",
                    i,
                    item.code,
                    bit_start,
                    item.bits.len()
                );
                if i == 0 {
                    // Peek at next bits using recorder
                    let next = recorder.read_bits_u64(64).unwrap_or(0);
                    println!("Next 64 bits from here: {:064b}", next);
                }
            }
            Err(e) => {
                println!("Error at Item {}: {}", i, e);
                break;
            }
        }
    }
}
