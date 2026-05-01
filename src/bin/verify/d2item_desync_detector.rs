use d2r_core::item::HuffmanTree;
use d2r_core::verify::args::{ArgParser, ArgSpec};
use d2r_core::verify::desync::detect_desync;
use std::env;
use std::fs;

fn main() {
    let mut parser = ArgParser::new("d2item_desync_detector");
    parser.add_spec(ArgSpec::positional("save_file", "Path to save file"));
    parser.add_spec(ArgSpec::flag("json", None, Some("json"), "Output results in JSON format"));

    use d2r_core::verify::args::ArgError;
    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            std::process::exit(0);
        }
        Err(ArgError::Error(e)) => {
            eprintln!("error: {}", e);
            eprintln!("\n{}", parser.usage());
            std::process::exit(1);
        }
    };

    let path = parsed.get("save_file").unwrap();
    let use_json = parsed.is_set("json");

    let bytes = match fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read file: {}", e);
            std::process::exit(1);
        }
    };

    let huffman = HuffmanTree::new();
    let is_alpha = bytes[4..8] == [0x69, 0, 0, 0];

    match detect_desync(&bytes, &huffman, is_alpha) {
        Ok(reports) => {
            if use_json {
                println!("{}", serde_json::to_string_pretty(&reports).unwrap());
            } else {
                println!("Cascading Desync Report for: {}", path);
                println!("Mode: {}", if is_alpha { "Alpha v105" } else { "Retail" });
                println!("{:-<80}", "");
                println!("{:>5} | {:>12} | {:>12} | {:>8} | {:>10} | {:>5}", "Index", "Oracle Start", "Parser Start", "Drift", "Code", "Match");
                println!("{:-<80}", "");
                
                let mut first_desync = None;
                for r in &reports {
                    println!("{:5} | {:12} | {:12} | {:8} | {:10} | {:5}", 
                        r.item_index, r.oracle_start, r.parser_start, r.drift, r.item_code, if r.is_match { "OK" } else { "FAIL" }
                    );
                    if !r.is_match && first_desync.is_none() {
                        first_desync = Some(r.item_index);
                    }
                }
                println!("{:-<80}", "");
                if let Some(idx) = first_desync {
                    println!("[ALERT] First desync detected at Item Index {}.", idx);
                } else {
                    println!("[PASS] No bitstream drift detected across {} items.", reports.len());
                }
            }
        }
        Err(e) => {
            eprintln!("Error during desync detection: {}", e);
            std::process::exit(1);
        }
    }
}
