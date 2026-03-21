use std::env;
use std::fs;
use std::io;

use d2r_core::save::{gf_payload_range, map_core_sections};

fn print_section_map(label: &str, map: &d2r_core::save::SaveSectionMap) {
    let jm_str: Vec<String> = map
        .jm_positions
        .iter()
        .enumerate()
        .map(|(idx, pos)| format!("JM#{idx}:{pos}"))
        .collect();
    println!(
        "{label}: gf@{} if@{} jm=[{}]",
        map.gf_pos,
        map.if_pos,
        jm_str.join(", ")
    );
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: d2save_status_diff <file_a.d2s> <file_b.d2s>");
        std::process::exit(1);
    }

    let bytes_a = fs::read(&args[1])?;
    let bytes_b = fs::read(&args[2])?;

    let map_a = map_core_sections(&bytes_a)?;
    let map_b = map_core_sections(&bytes_b)?;

    println!("=== STATUS DIFF ===");
    print_section_map("File A", &map_a);
    print_section_map("File B", &map_b);

    let range_a = gf_payload_range(&map_a);
    let range_b = gf_payload_range(&map_b);
    println!(
        "GF payload lengths: A={} B={}",
        range_a.len(),
        range_b.len()
    );

    let min_len = range_a.len().min(range_b.len());
    let mut diffs = Vec::new();
    for offset in 0..min_len {
        let a_val = bytes_a[range_a.start + offset];
        let b_val = bytes_b[range_b.start + offset];
        if a_val != b_val {
            diffs.push((range_a.start + offset, a_val, b_val));
            if diffs.len() >= 40 {
                break;
            }
        }
    }

    println!("First {} GF diffs:", diffs.len());
    for (idx, (pos, a_val, b_val)) in diffs.iter().enumerate() {
        println!(
            "  #{idx}: offset {pos} -> A=0x{a_val:02X} B=0x{b_val:02X}"
        );
    }

    if range_a.len() != range_b.len() {
        println!(
            "GF length mismatch: A={} B={}",
            range_a.len(),
            range_b.len()
        );
    }

    Ok(())
}
