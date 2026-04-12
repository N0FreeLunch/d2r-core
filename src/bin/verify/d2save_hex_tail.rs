use d2r_core::verify::args::{ArgError, ArgParser, ArgSpec};
use std::env;
use std::fs;

fn main() {
    let mut parser = ArgParser::new("d2save_hex_tail")
        .description("Dumps the last 64 bytes of a .d2s save file (hex tail)");
    parser.add_spec(ArgSpec::positional("file", "Path to the .d2s save file"));

    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            return;
        }
        Err(ArgError::Error(e)) => {
            eprintln!("error: {}\n\n{}", e, parser.usage());
            std::process::exit(1);
        }
    };

    let file_path = parsed.get("file").unwrap();
    let bytes = fs::read(file_path).unwrap();
    let len = bytes.len();
    let start = len.saturating_sub(64);

    println!("Last {} bytes of {}:", len - start, file_path);
    for i in start..len {
        print!("{:02X} ", bytes[i]);
        if (i + 1) % 16 == 0 {
            println!();
        }
    }
    println!();
}
