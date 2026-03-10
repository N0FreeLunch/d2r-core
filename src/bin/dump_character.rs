use d2r_core::item::{HuffmanTree, Item};
use d2r_core::save::{Save, map_core_sections, parse_skill_section};
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: dump_character <file.d2s>");
        return;
    }

    let bytes = fs::read(&args[1]).expect("Failed to read file");
    let save = Save::from_bytes(&bytes).expect("Failed to parse header");
    let huffman = HuffmanTree::new();

    println!("Character: {}", save.header.char_name);
    println!("Level:     {}", save.header.char_level);
    println!(
        "Class:     {}",
        d2r_core::save::class_name(save.header.char_class)
    );

    if let Ok(map) = map_core_sections(&bytes) {
        println!(
            "Markers: gf={}, if={}, JM_count={}",
            map.gf_pos,
            map.if_pos,
            map.jm_positions.len()
        );
        for (i, &pos) in map.jm_positions.iter().enumerate() {
            let count = u16::from_le_bytes([bytes[pos + 2], bytes[pos + 3]]);
            println!("  JM Section {}: offset={}, count={}", i, pos, count);
        }

        if let Ok(skills) = parse_skill_section(&bytes, &map) {
            println!("Skills:");
            for (i, &lvl) in skills.as_slice().iter().enumerate() {
                if lvl > 0 {
                    println!("  Index {}: Level {}", i, lvl);
                }
            }
        }
    }

    if let Ok(items) = Item::read_player_items(&bytes, &huffman) {
        println!("Player Items: {}", items.len());
        for (i, item) in items.iter().enumerate() {
            println!(
                "  [{:>2}] code={} mode={} loc={} x={} y={} page={} sockets={}",
                i,
                item.code,
                item.mode,
                item.location,
                item.x,
                item.y,
                item.page,
                item.socketed_items.len()
            );
            for (si, child) in item.socketed_items.iter().enumerate() {
                println!("     Socket {}: code={}", si, child.code);
            }
        }
    }
}
