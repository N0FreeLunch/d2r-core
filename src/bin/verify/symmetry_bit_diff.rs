use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;
use d2r_core::verify::args::{ArgParser, ArgSpec, ArgError};

fn main() {
    unsafe { std::env::set_var("D2R_ITEM_TRACE", "1"); }
    let mut parser = ArgParser::new("SymmetryBitDiff")
        .description("Compares item-by-item bitstream symmetry. Supports memory roundtrip for a single file.");

    parser.add_spec(ArgSpec::positional("file_a", "path to the save file (.d2s)"));
    parser.add_spec(ArgSpec::positional("file_b", "path to the second save file (.d2s)").optional());
    parser.add_spec(ArgSpec::flag("roundtrip", Some('r'), Some("roundtrip"), "if set, compares file_a with its own reserialized items"));

    let args: Vec<_> = env::args_os().skip(1).collect();
    let parsed = match parser.parse(args) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            process::exit(0);
        }
        Err(ArgError::Error(e)) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };

    let path_a = parsed.get("file_a").unwrap();
    let is_roundtrip = parsed.is_set("roundtrip");

    let bytes_a = fs::read(path_a).expect("failed to read save file A");
    let huffman = HuffmanTree::new();
    let version_a = u32::from_le_bytes(bytes_a[4..8].try_into().unwrap_or([0; 4]));
    let is_alpha_a = version_a == 105 || version_a == 6;

    if is_roundtrip {
        println!("Performing memory roundtrip analysis for A...");
        let items = Item::read_player_items(&bytes_a, &huffman, is_alpha_a).expect("failed to read items from A");
        println!("  - Recovered {} top-level items", items.len());

        for (i, item) in items.iter().enumerate() {
            compare_item_with_reserialized(item, &huffman, is_alpha_a, format!("Item {}", i), 0);
        }
    } else {
        let path_b = parsed.get("file_b").expect("file_b is required when --roundtrip is not set");
        let bytes_b = fs::read(path_b).expect("failed to read save file B");
        let version_b = u32::from_le_bytes(bytes_b[4..8].try_into().unwrap_or([0; 4]));
        let is_alpha_b = version_b == 105 || version_b == 6;

        let items_a = Item::read_player_items(&bytes_a, &huffman, is_alpha_a).expect("failed to read items from A");
        let items_b = Item::read_player_items(&bytes_b, &huffman, is_alpha_b).expect("failed to read items from B");

        println!("Comparing {} items from A with {} items from B...", items_a.len(), items_b.len());
        for i in 0..items_a.len().min(items_b.len()) {
            compare_two_items(&items_a[i], &items_b[i], format!("Item {}", i), 0);
        }
    }
}

fn compare_item_with_reserialized(item: &Item, huffman: &HuffmanTree, alpha_mode: bool, prefix: String, depth: usize) {
    let indent = "  ".repeat(depth);
    let reserialized_bytes = item.to_bytes(huffman, alpha_mode).expect("failed to reserialize");
    
    // Original bits from parsing
    let original_bits = &item.bits;
    
    // We need to convert reserialized_bytes back to bit vector for comparison
    let mut rebuilt_bits = Vec::new();
    let mut reader = IoBitReader::endian(Cursor::new(&reserialized_bytes), LittleEndian);
    for _ in 0..original_bits.len() {
        if let Ok(bit) = reader.read_bit() {
            rebuilt_bits.push(bit);
        } else {
            break;
        }
    }

    print!("{}{} match: '{}'", indent, prefix, item.code.trim());
    
    let mut mismatch_idx = None;
    for i in 0..original_bits.len().min(rebuilt_bits.len()) {
        if original_bits[i].bit != rebuilt_bits[i] {
            mismatch_idx = Some(i);
            break;
        }
    }

    if mismatch_idx.is_some() || original_bits.len() != rebuilt_bits.len() {
        println!(" [DIFF]");
        println!("{}  Length: Original={} bits, Rebuilt={} bits", indent, original_bits.len(), rebuilt_bits.len());
        if let Some(idx) = mismatch_idx {
            let offset = original_bits[idx].offset;
            let segment_name = find_segment_for_offset(item, offset).unwrap_or_else(|| "Unknown".to_string());
            println!("{}  First mismatch at bit offset {} (Segment: {})", indent, idx, segment_name);
        }
    } else {
        println!(" ({} bits)", original_bits.len());
    }

    for (i, child) in item.socketed_items.iter().enumerate() {
        compare_item_with_reserialized(child, huffman, alpha_mode, format!("Child {}", i), depth + 1);
    }
}

fn compare_two_items(item_a: &Item, item_b: &Item, prefix: String, depth: usize) {
    let indent = "  ".repeat(depth);
    print!("{}{} match: '{}' vs '{}'", indent, prefix, item_a.code.trim(), item_b.code.trim());

    if item_a.bits.len() != item_b.bits.len() {
        println!(" [DIFF] Length mismatch (A={} bits, B={} bits)", item_a.bits.len(), item_b.bits.len());
    } else {
        let mut mismatch_idx = None;
        for i in 0..item_a.bits.len() {
            if item_a.bits[i].bit != item_b.bits[i].bit {
                mismatch_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = mismatch_idx {
            let offset = item_a.bits[idx].offset;
            let segment_name = find_segment_for_offset(item_a, offset).unwrap_or_else(|| "Unknown".to_string());
            println!(" [DIFF] Content mismatch at bit offset {} (Segment: {})", idx, segment_name);
        } else {
            println!(" ({} bits)", item_a.bits.len());
        }
    }

    for i in 0..item_a.socketed_items.len().max(item_b.socketed_items.len()) {
        if i < item_a.socketed_items.len() && i < item_b.socketed_items.len() {
            compare_two_items(&item_a.socketed_items[i], &item_b.socketed_items[i], format!("Child {}", i), depth + 1);
        } else {
            println!("{}  Child count mismatch at index {}", indent, i);
        }
    }
}

fn find_segment_for_offset(item: &Item, offset: u64) -> Option<String> {
    // Find the deepest segment that contains this offset
    let mut best_segment: Option<&crate::domain::item::entity::BitSegment> = None;
    
    for seg in &item.segments {
        if offset >= seg.start && offset < seg.end {
            if let Some(best) = best_segment {
                if seg.depth > best.depth {
                    best_segment = Some(seg);
                }
            } else {
                best_segment = Some(seg);
            }
        }
    }

    if let Some(seg) = best_segment {
        return Some(seg.label.clone());
    }

    // Check children recursively
    for child in &item.socketed_items {
        if let Some(name) = find_segment_for_offset(child, offset) {
            return Some(format!("{} -> {}", item.code.trim(), name));
        }
    }
    None
}
