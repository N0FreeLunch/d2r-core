use std::env;
use std::fs;
use std::process;

fn find_first_jm(bytes: &[u8]) -> Option<usize> {
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'J' && bytes[i + 1] == b'M' {
            return Some(i);
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: d2save_item_diff <file_a.d2s> <file_b.d2s>");
        process::exit(1);
    }

    let path_a = &args[1];
    let path_b = &args[2];

    let bytes_a = match fs::read(path_a) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[ERROR] Cannot read '{}': {}", path_a, e);
            process::exit(1);
        }
    };
    let bytes_b = match fs::read(path_b) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("[ERROR] Cannot read '{}': {}", path_b, e);
            process::exit(1);
        }
    };

    println!("=== d2save_item_diff (JM Aligned) ===");
    println!("  A: {} ({} bytes)", path_a, bytes_a.len());
    println!("  B: {} ({} bytes)", path_b, bytes_b.len());

    let jm_a_idx = find_first_jm(&bytes_a);
    let jm_b_idx = find_first_jm(&bytes_b);

    if jm_a_idx.is_none() || jm_b_idx.is_none() {
        eprintln!("[ERROR] Missing JM in one or both files.");
        process::exit(1);
    }

    let a_start = jm_a_idx.unwrap();
    let b_start = jm_b_idx.unwrap();

    println!("  First JM in A: offset {}", a_start);
    println!("  First JM in B: offset {}", b_start);

    let items_a = &bytes_a[a_start..];
    let items_b = &bytes_b[b_start..];

    let common_len = items_a.len().min(items_b.len());
    let mut diffs = Vec::new();

    for i in 0..common_len {
        if items_a[i] != items_b[i] {
            diffs.push(i);
        }
    }

    println!();
    println!("[ITEM SECTION DIFF SUMMARY]");
    println!("  Items A length: {} bytes", items_a.len());
    println!("  Items B length: {} bytes", items_b.len());

    if items_a.len() != items_b.len() {
        println!(
            "  Length diff: {} bytes",
            items_b.len() as isize - items_a.len() as isize
        );
    }

    println!("  Total differences: {} bytes", diffs.len());

    if diffs.is_empty() && items_a.len() == items_b.len() {
        println!("\n  ✅ [IDENTICAL] The Item Sections (JM onwards) are 100% strictly identical.");
        process::exit(0);
    }

    println!();
    println!("[DETAILS] (first 30 differences in Item Section)");
    println!(
        "  {:>10}  {:>10}  {:>10}  {:>10}",
        "Rel Offset", "Abs A", "A (hex)", "B (hex)"
    );
    println!("  {:->10}  {:->10}  {:->10}  {:->10}", "", "", "", "");
    for &i in diffs.iter().take(30) {
        let a_val = items_a[i];
        let b_val = items_b[i];
        let abs_a = a_start + i;
        println!(
            "  {:>10}  {:>10}  0x{:02X} ({:>3})   0x{:02X} ({:>3})",
            i, abs_a, a_val, a_val, b_val, b_val
        );
    }
    if diffs.len() > 30 {
        println!("  ... and {} more diffs", diffs.len() - 30);
    }

    process::exit(1);
}
