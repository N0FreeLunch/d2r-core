use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: d2save_hex_tail <file.d2s>");
        return;
    }

    let bytes = fs::read(&args[1]).unwrap();
    let len = bytes.len();
    let start = len.saturating_sub(64);

    println!("Last {} bytes of {}:", len - start, args[1]);
    for i in start..len {
        print!("{:02X} ", bytes[i]);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
}
