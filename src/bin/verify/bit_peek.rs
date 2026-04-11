use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::{HuffmanTree, Item};
use d2r_core::verify::args::{ArgParser, ArgSpec};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let mut parser = ArgParser::new("d2item_bit_peek");
    parser.add_spec(ArgSpec::positional("save_file", "Path to save file"));
    parser.add_spec(
        ArgSpec::positional("offset", "Bit offset from start")
            .optional()
            .with_default("0"),
    );
    parser.add_spec(
        ArgSpec::positional("count_bits", "Number of bits to read")
            .optional()
            .with_default("64"),
    );

    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            eprintln!("\n{}", parser.usage());
            std::process::exit(1);
        }
    };

    let path = parsed.get("save_file").unwrap();
    let offset = parsed
        .get("offset")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let count_bits = parsed
        .get("count_bits")
        .and_then(|s| s.parse::<u32>().ok())
        .unwrap_or(64);
    let bytes = fs::read(path).expect("failed to read save file");

    if offset > 0 {
        let mut reader =
            IoBitReader::endian(Cursor::new(&bytes[(offset / 8) as usize..]), LittleEndian);
        let _ = reader.skip((offset % 8) as u32);
        let mut cursor = BitCursor::new(reader);
        let val: u64 = cursor.read_bits::<u64>(count_bits).unwrap_or(0);
        println!(
            "Bits at offset {}: {:0width$b}",
            offset,
            val,
            width = count_bits as usize
        );
        return;
    }

    let jm_pos = (0..bytes.len() - 2)
        .find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M')
        .expect("No JM marker found");
    let count = u16::from_le_bytes([bytes[jm_pos + 2], bytes[jm_pos + 3]]);
    println!("JM at byte {}, item count {}", jm_pos, count);

    let huffman = HuffmanTree::new();
    let reader = IoBitReader::endian(Cursor::new(&bytes[jm_pos + 4..]), LittleEndian);
    let mut recorder = BitCursor::new(reader);

    let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];
    for i in 0..count {
        let bit_start = (jm_pos + 4) * 8 + recorder.pos() as usize;
        match Item::from_reader_with_context(
            &mut recorder,
            &huffman,
            Some((&bytes, ((jm_pos + 4) * 8) as u64)),
            is_alpha,
        ) {
            Ok(item) => {
                println!(
                    "Item {}: '{}' (start_bit={}, len={} bits)",
                    i,
                    item.code,
                    bit_start,
                    item.bits.len()
                );

                // Forensic BitRange Verification (Slice S2)
                let range_bits = item.range.end - item.range.start;
                if range_bits != item.bits.len() as u64 {
                    println!("  [FATAL] BitRange desync: Item reported {} bits, but bits.len() is {}", range_bits, item.bits.len());
                } else {
                    println!("  [PASS] BitRange consistent: {} bits", range_bits);
                }

                println!("  BSLV Layout Tree:");
                let mut segments = recorder.segments().to_vec();
                // Sort by start bit (asc) and then by length (desc) to keep parents outside children
                segments.sort_by(|a, b| {
                    a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end))
                });

                for seg in segments {
                    if seg.start == seg.end && seg.label == "Item Code" {
                        continue; // Skip noise from empty codes
                    }
                    let indent = "    ".repeat(seg.depth);
                    let len = seg.end - seg.start;
                    println!("  {}[{:>4}..{:>4}] (len={:>2}) {}", indent, seg.start, seg.end, len, seg.label);
                }

                if i == 0 {
                    // Peek at next bits using recorder
                    let next: u64 = recorder.read_bits::<u64>(64).unwrap_or(0);
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
