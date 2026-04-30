use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;
use d2r_core::verify::args::{ArgParser, ArgSpec, ArgError};
use serde::Serialize;

#[derive(Serialize, Default)]
struct DiffReport {
    success: bool,
    operation: String,
    item_count_a: usize,
    item_count_b: usize,
    items: Vec<ItemDiff>,
}

#[derive(Serialize, Default)]
struct ItemDiff {
    label: String,
    code: String,
    is_match: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    mismatch_type: Option<String>,
    original_len: usize,
    target_len: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    first_mismatch_offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    segment: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    children: Vec<ItemDiff>,
}

fn main() {
    unsafe { std::env::set_var("D2R_ITEM_TRACE", "1"); }
    let mut parser = ArgParser::new("SymmetryBitDiff")
        .description("Compares item-by-item bitstream symmetry. Supports memory roundtrip for a single file.");

    parser.add_spec(ArgSpec::positional("file_a", "path to the save file (.d2s)"));
    parser.add_spec(ArgSpec::positional("file_b", "path to the second save file (.d2s)").optional());
    parser.add_spec(ArgSpec::flag("roundtrip", Some('r'), Some("roundtrip"), "if set, compares file_a with its own reserialized items"));
    parser.add_spec(ArgSpec::flag("json", Some('j'), Some("json"), "if set, outputs results in JSON format"));

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
    let is_json = parsed.is_set("json");

    let bytes_a = fs::read(path_a).expect("failed to read save file A");
    let huffman = HuffmanTree::new();
    let version_a = u32::from_le_bytes(bytes_a[4..8].try_into().unwrap_or([0; 4]));
    let is_alpha_a = version_a == 105 || version_a == 6;

    let mut report = DiffReport {
        operation: if is_roundtrip { "roundtrip".to_string() } else { "compare".to_string() },
        ..Default::default()
    };

    if is_roundtrip {
        if !is_json {
            println!("Performing memory roundtrip analysis for A...");
        }
        let items = Item::read_player_items(&bytes_a, &huffman, is_alpha_a).expect("failed to read items from A");
        report.item_count_a = items.len();
        report.item_count_b = items.len(); // roundtrip always has same count
        if !is_json {
            println!("  - Recovered {} top-level items", items.len());
        }

        for (i, item) in items.iter().enumerate() {
            let item_diff = compare_item_with_reserialized(item, &huffman, is_alpha_a, format!("Item {}", i), 0, is_json);
            report.items.push(item_diff);
        }
    } else {
        let path_b = parsed.get("file_b").expect("file_b is required when --roundtrip is not set");
        let bytes_b = fs::read(path_b).expect("failed to read save file B");
        let version_b = u32::from_le_bytes(bytes_b[4..8].try_into().unwrap_or([0; 4]));
        let is_alpha_b = version_b == 105 || version_b == 6;

        let items_a = Item::read_player_items(&bytes_a, &huffman, is_alpha_a).expect("failed to read items from A");
        let items_b = Item::read_player_items(&bytes_b, &huffman, is_alpha_b).expect("failed to read items from B");
        report.item_count_a = items_a.len();
        report.item_count_b = items_b.len();

        if !is_json {
            println!("Comparing {} items from A with {} items from B...", items_a.len(), items_b.len());
        }
        for i in 0..items_a.len().min(items_b.len()) {
            let item_diff = compare_two_items(&items_a[i], &items_b[i], format!("Item {}", i), 0, is_json);
            report.items.push(item_diff);
        }
    }

    report.success = report.item_count_a == report.item_count_b && report.items.iter().all(|i| i.is_match);
    if is_json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    }
}

fn compare_item_with_reserialized(item: &Item, huffman: &HuffmanTree, alpha_mode: bool, prefix: String, depth: usize, is_json: bool) -> ItemDiff {
    let indent = "  ".repeat(depth);
    
    // Force strict reconstruction by clearing bits in a clone
    let mut strict_item = item.clone();
    strict_item.bits.clear();
    let reserialized_bytes = strict_item.to_bytes(huffman, alpha_mode).expect("failed to reserialize");
    
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

    if !is_json {
        print!("{}{} match: '{}'", indent, prefix, item.code.trim());
    }
    
    let mut mismatch_idx = None;
    for i in 0..original_bits.len().min(rebuilt_bits.len()) {
        if original_bits[i].bit != rebuilt_bits[i] {
            mismatch_idx = Some(i);
            break;
        }
    }

    let mut item_diff = ItemDiff {
        label: prefix,
        code: item.code.trim().to_string(),
        original_len: original_bits.len(),
        target_len: rebuilt_bits.len(),
        ..Default::default()
    };

    if mismatch_idx.is_some() || original_bits.len() != rebuilt_bits.len() {
        item_diff.is_match = false;
        if original_bits.len() != rebuilt_bits.len() {
            item_diff.mismatch_type = Some("Length".to_string());
        } else {
            item_diff.mismatch_type = Some("Content".to_string());
        }

        if !is_json {
            println!(" [DIFF]");
            println!("{}  Length: Original={} bits, Rebuilt={} bits", indent, original_bits.len(), rebuilt_bits.len());
        }
        if let Some(idx) = mismatch_idx {
            item_diff.first_mismatch_offset = Some(idx as u64);
            let segment_name = item.query_bit(idx as u64).map(|s| s.label).unwrap_or_else(|| "Unknown".to_string());
            item_diff.segment = Some(segment_name.clone());
            if !is_json {
                println!("{}  First mismatch at bit offset {} (Segment: {})", indent, idx, segment_name);
            }
        }
    } else {
        item_diff.is_match = true;
        if !is_json {
            println!(" ({} bits)", original_bits.len());
        }
    }

    for (i, child) in item.socketed_items.iter().enumerate() {
        let child_diff = compare_item_with_reserialized(child, huffman, alpha_mode, format!("Child {}", i), depth + 1, is_json);
        item_diff.children.push(child_diff);
    }
    
    // If any child doesn't match, this item doesn't match either
    if !item_diff.children.iter().all(|c| c.is_match) {
        item_diff.is_match = false;
    }

    item_diff
}

fn compare_two_items(item_a: &Item, item_b: &Item, prefix: String, depth: usize, is_json: bool) -> ItemDiff {
    let indent = "  ".repeat(depth);
    if !is_json {
        print!("{}{} match: '{}' vs '{}'", indent, prefix, item_a.code.trim(), item_b.code.trim());
    }

    let mut item_diff = ItemDiff {
        label: prefix.clone(),
        code: item_a.code.trim().to_string(),
        original_len: item_a.bits.len(),
        target_len: item_b.bits.len(),
        ..Default::default()
    };

    if item_a.bits.len() != item_b.bits.len() {
        item_diff.is_match = false;
        item_diff.mismatch_type = Some("Length".to_string());
        if !is_json {
            println!(" [DIFF] Length mismatch (A={} bits, B={} bits)", item_a.bits.len(), item_b.bits.len());
        }
    } else {
        let mut mismatch_idx = None;
        for i in 0..item_a.bits.len() {
            if item_a.bits[i].bit != item_b.bits[i].bit {
                mismatch_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = mismatch_idx {
            item_diff.is_match = false;
            item_diff.mismatch_type = Some("Content".to_string());
            item_diff.first_mismatch_offset = Some(idx as u64);
            let segment_name = item_a.query_bit(idx as u64).map(|s| s.label).unwrap_or_else(|| "Unknown".to_string());
            item_diff.segment = Some(segment_name.clone());
            if !is_json {
                println!(" [DIFF] Content mismatch at bit offset {} (Segment: {})", idx, segment_name);
            }
        } else {
            item_diff.is_match = true;
            if !is_json {
                println!(" ({} bits)", item_a.bits.len());
            }
        }
    }

    for i in 0..item_a.socketed_items.len().max(item_b.socketed_items.len()) {
        if i < item_a.socketed_items.len() && i < item_b.socketed_items.len() {
            let child_diff = compare_two_items(&item_a.socketed_items[i], &item_b.socketed_items[i], format!("Child {}", i), depth + 1, is_json);
            item_diff.children.push(child_diff);
        } else {
            item_diff.is_match = false;
            let mut child_diff = ItemDiff {
                label: format!("Child {}", i),
                is_match: false,
                mismatch_type: Some("ChildCount".to_string()),
                ..Default::default()
            };
            if i < item_a.socketed_items.len() {
                child_diff.code = item_a.socketed_items[i].code.trim().to_string();
                child_diff.original_len = item_a.socketed_items[i].bits.len();
            } else {
                child_diff.code = item_b.socketed_items[i].code.trim().to_string();
                child_diff.target_len = item_b.socketed_items[i].bits.len();
            }
            item_diff.children.push(child_diff);
            if !is_json {
                println!("{}  Child count mismatch at index {}", indent, i);
            }
        }
    }
    
    if !item_diff.children.iter().all(|c| c.is_match) {
        item_diff.is_match = false;
    }

    item_diff
}
