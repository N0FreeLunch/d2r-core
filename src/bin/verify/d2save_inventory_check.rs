use d2r_core::inventory::InventoryGrid;
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2save_inventory_check <file.d2s>");
        process::exit(1);
    }

    let path = &args[1];
    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[ERROR] Cannot read '{}': {}", path, e);
            process::exit(1);
        }
    };

    println!("=== Inventory Integrity Check: {} ===", path);

    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    let huffman = HuffmanTree::new();

    // Find all JM markers
    let mut jm_positions: Vec<usize> = Vec::new();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'J' && bytes[i + 1] == b'M' {
            jm_positions.push(i);
        }
    }

    if jm_positions.is_empty() {
        println!("[WARN] No JM markers found.");
        return;
    }

    // Usually the first JM is Player Items
    let first_jm = jm_positions[0];
    let item_count = u16::from_le_bytes([bytes[first_jm + 2], bytes[first_jm + 3]]);

    let mut reader = bitstream_io::BitReader::endian(
        std::io::Cursor::new(&bytes[first_jm + 4..]),
        bitstream_io::LittleEndian,
    );

    let mut items = Vec::new();
    for _ in 0..item_count {
        let _ = bitstream_io::BitRead::byte_align(&mut reader);
        if let Ok(item) = Item::from_reader(&mut reader, &huffman, version == 105) {
            items.push(item);
        } else {
            break;
        }
    }

    println!("  Analyzing {} items in Player section...", items.len());

    for (i, item) in items.iter().enumerate() {
        let category = d2r_core::inventory::get_item_category(&item.code);
        println!(
            "  - Item[{:>2}]: code='{}' -> category='{}'",
            i, item.code, category
        );
    }
    println!();

    let errors = InventoryGrid::validate_logical_integrity(&items, 10, 4);

    if errors.is_empty() {
        println!("\x1b[32m[OK] No inventory collisions or out-of-bounds detected.\x1b[0m");
    } else {
        println!(
            "\x1b[31m[FAILED] Found {} inventory errors:\x1b[0m",
            errors.len()
        );
        for (i, err) in errors.iter().enumerate() {
            println!("  {:>2}. {}", i + 1, err);
        }
    }

    println!("\n[Final Inventory Layout]");
    let grid = InventoryGrid::from_save_bytes(&bytes, &huffman);
    grid.debug_print();
}
