use std::env;
use std::fs;
use std::io::{self, Read};

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: diff <file1> <file2>");
        return Ok(());
    }

    let file1_path = &args[1];
    let file2_path = &args[2];

    let mut f1 = fs::File::open(file1_path)?;
    let mut f2 = fs::File::open(file2_path)?;

    let mut bytes1 = Vec::new();
    let mut bytes2 = Vec::new();

    f1.read_to_end(&mut bytes1)?;
    f2.read_to_end(&mut bytes2)?;

    println!(
        "Comparing {} ({}) and {} ({})",
        file1_path,
        bytes1.len(),
        file2_path,
        bytes2.len()
    );

    let max_len = bytes1.len().max(bytes2.len());

    for i in 0..max_len {
        let b1 = bytes1.get(i);
        let b2 = bytes2.get(i);

        if b1 != b2 {
            let hex1 = b1
                .map(|b| format!("{:02x}", b))
                .unwrap_or_else(|| "--".to_string());
            let hex2 = b2
                .map(|b| format!("{:02x}", b))
                .unwrap_or_else(|| "--".to_string());
            println!("Offset 0x{:04x} ({}): {} -> {}", i, i, hex1, hex2);
        }
    }

    Ok(())
}
