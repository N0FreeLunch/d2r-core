use std::env;
use std::fs;
use std::io;

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut file1_path = String::new();
    let mut file2_path = String::new();
    let mut start_offset = 415; // Default for v105 Quest anchor probe
    let mut stride = 4; // Default for v105 Quest stride probe

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--start" => {
                i += 1;
                if i < args.len() {
                    start_offset = args[i].parse().unwrap_or(415);
                }
            }
            "--stride" => {
                i += 1;
                if i < args.len() {
                    stride = args[i].parse().unwrap_or(4);
                }
            }
            "--help" | "-h" => {
                println!("Usage: cargo run --example diff -- [options] <file1> <file2>");
                println!("Options:");
                println!(
                    "  --start <N>   Absolute offset to start relative numbering (default: 415)"
                );
                println!("  --stride <N>  Grouping size for relative numbering (default: 4)");
                return Ok(());
            }
            _ => {
                if file1_path.is_empty() {
                    file1_path = args[i].clone();
                } else if file2_path.is_empty() {
                    file2_path = args[i].clone();
                }
            }
        }
        i += 1;
    }

    if file1_path.is_empty() || file2_path.is_empty() {
        println!("Usage: cargo run --example diff -- [--start <N>] [--stride <N>] <file1> <file2>");
        return Ok(());
    }

    let bytes1 = fs::read(&file1_path)?;
    let bytes2 = fs::read(&file2_path)?;

    println!(
        "Comparing {} ({}) and {} ({})",
        file1_path,
        bytes1.len(),
        file2_path,
        bytes2.len()
    );
    println!(
        "Probe Config | Start: {} (0x{:04X}), Stride: {}",
        start_offset, start_offset, stride
    );
    println!("Offset      | Hex Diff   | Binary Diff             | Rel");
    println!("------------|------------|-------------------------|----");

    let max_len = bytes1.len().max(bytes2.len());

    for i in 0..max_len {
        let b1 = bytes1.get(i);
        let b2 = bytes2.get(i);

        if b1 != b2 {
            let hex1 = b1
                .map(|b| format!("{:02X}", b))
                .unwrap_or_else(|| "--".to_string());
            let hex2 = b2
                .map(|b| format!("{:02X}", b))
                .unwrap_or_else(|| "--".to_string());

            let bin1 = b1
                .map(|b| format!("{:08b}", b))
                .unwrap_or_else(|| "--------".to_string());
            let bin2 = b2
                .map(|b| format!("{:08b}", b))
                .unwrap_or_else(|| "--------".to_string());

            let rel = if i >= start_offset {
                format!("+{}", (i - start_offset) % stride)
            } else {
                "-".to_string()
            };

            println!(
                "0x{:04X} ({:>4}) | {} -> {} | {} -> {} | {}",
                i, i, hex1, hex2, bin1, bin2, rel
            );
        }
    }

    Ok(())
}
