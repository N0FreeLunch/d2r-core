use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use d2r_core::data::bit_cursor::BitCursor;
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;
use d2r_core::verify::args::{ArgParser, ArgSpec, ArgError};

fn main() {
    let mut parser = ArgParser::new("d2item_bit_search")
        .description("Scans for property terminator (0x1FF) around a specific bit offset in a D2R save file");

    parser.add_spec(ArgSpec::positional("save_file", "path to the save file (.d2s)"));
    parser.add_spec(ArgSpec::positional("offset_bits", "starting bit offset to probe around"));

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

    let save_file = parsed.get("save_file").unwrap();
    let start_bit: u64 = parsed.get("offset_bits").and_then(|s| s.parse().ok()).expect("offset_bits must be a valid number");
    
    let bytes = fs::read(save_file).expect("failed to read save file");

    println!("Scanning for property terminator (0x1FF) around bit {}:", start_bit);

    for nudge in -32i64..=128i64 {
        let current = (start_bit as i64 + nudge) as u64;
        if current >= (bytes.len() * 8) as u64 { continue; }

        let mut reader = IoBitReader::endian(Cursor::new(&bytes[(current / 8) as usize..]), LittleEndian);      
        let mut recorder = BitCursor::new(&mut reader);
        let _ = recorder.skip_and_record((current % 8) as u32);

        let id: u32 = recorder.read_bits::<u32>(9).unwrap_or(0);
        if id == 0x1FF {
            println!("  [Bit {}] Terminator found! (nudge {})", current, nudge);
        }
    }
}
