use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::verify::args::{ArgError, ArgParser, ArgSpec};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn main() {
    let mut parser = ArgParser::new("d2item_bit_dump")
        .description("Dumps raw bits from a save file starting at a base bit offset in a visual matrix format.");
    parser.add_spec(ArgSpec::positional("save_file", "Path to save file"));
    parser.add_spec(ArgSpec::positional("base_bit", "Starting bit offset"));
    parser.add_spec(
        ArgSpec::positional("rows", "Number of rows (default: 16)")
            .optional()
            .with_default("16"),
    );
    parser.add_spec(
        ArgSpec::positional("width", "Bit width of each row (default: 9)")
            .optional()
            .with_default("9"),
    );

    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            return;
        }
        Err(ArgError::Error(e)) => {
            eprintln!("error: {}\n\n{}", e, parser.usage());
            process::exit(1);
        }
    };

    let path = parsed.get("save_file").unwrap();
    let base_bit: usize = parsed
        .get("base_bit")
        .unwrap()
        .parse()
        .expect("base_bit must be a number");
    let rows: usize = parsed
        .get("rows")
        .and_then(|s| s.parse().ok())
        .unwrap_or(16);
    let width: usize = parsed
        .get("width")
        .and_then(|s| s.parse().ok())
        .unwrap_or(9);

    let bytes = fs::read(path).expect("failed to read save file");
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);

    if reader.skip(base_bit as u32).is_err() {
        eprintln!(
            "Error: Cannot skip to bit {} (outside file boundaries).",
            base_bit
        );
        process::exit(1);
    }

    println!("Dumping raw bits from {}...", base_bit);
    println!("Visual Matrix: {} rows x {} width", rows, width);
    println!("------------------------------------------------------------");

    for r in 0..rows {
        let mut row_str = String::new();
        for _ in 0..width {
            match reader.read_bit() {
                Ok(bit) => row_str.push(if bit { '1' } else { '0' }),
                Err(_) => break,
            }
        }
        if row_str.is_empty() {
            break;
        }
        let current_pos = base_bit + (r + 1) * width;
        println!("{} (pos={})", row_str, current_pos);
    }
    println!("------------------------------------------------------------");
}
