use std::env;
use std::fs;
use std::process;
use d2r_core::verify::args::{ArgParser, ArgSpec, ArgError};
use serde::Serialize;

#[derive(Serialize)]
struct DsaReport {
    file_a: String,
    file_b: String,
    allowed_bits: Vec<usize>,
    identical: bool,
    violations: Vec<BitViolation>,
}

#[derive(Serialize)]
struct BitViolation {
    abs_bit: usize,
    byte_offset: usize,
    bit_in_byte: usize,
    val_a: u8,
    val_b: u8,
}

fn main() {
    let mut parser = ArgParser::new("d2save_dsa")
        .description("Domain Symmetry Auditor: Validates bit-level symmetry between two save files with allowed drift rules.");

    parser.add_spec(ArgSpec::positional("file_a", "path to the first save file (.d2s)"));
    parser.add_spec(ArgSpec::positional("file_b", "path to the second save file (.d2s)"));
    parser.add_spec(ArgSpec::option("allowed-bits", None, Some("allowed-bits"), "comma-separated list of allowed bit offsets (e.g. 81,96,108)"));

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

    let path_a = parsed.get("file_a").unwrap();
    let path_b = parsed.get("file_b").unwrap();
    let allowed_bits_str = parsed.get("allowed-bits").map(|s| s.as_str()).unwrap_or("");
    let is_json = parsed.is_json();

    let allowed_bits: Vec<usize> = if allowed_bits_str.is_empty() {
        Vec::new()
    } else {
        allowed_bits_str
            .split(',')
            .filter_map(|s| s.trim().parse::<usize>().ok())
            .collect()
    };

    let bytes_a = match fs::read(path_a) {
        Ok(b) => b,
        Err(e) => {
            if is_json {
                println!("{}", serde_json::json!({"error": format!("Cannot read '{}': {}", path_a, e)}));
            } else {
                eprintln!("[ERROR] Cannot read '{}': {}", path_a, e);
            }
            process::exit(1);
        }
    };
    let bytes_b = match fs::read(path_b) {
        Ok(b) => b,
        Err(e) => {
            if is_json {
                println!("{}", serde_json::json!({"error": format!("Cannot read '{}': {}", path_b, e)}));
            } else {
                eprintln!("[ERROR] Cannot read '{}': {}", path_b, e);
            }
            process::exit(1);
        }
    };

    if bytes_a.len() != bytes_b.len() {
        if is_json {
            println!("{}", serde_json::to_string_pretty(&serde_json::json!({
                "error": "Length mismatch",
                "len_a": bytes_a.len(),
                "len_b": bytes_b.len()
            })).unwrap());
        } else {
            eprintln!("[ERROR] Length mismatch: A={} bytes, B={} bytes", bytes_a.len(), bytes_b.len());
        }
        process::exit(1);
    }

    let mut violations = Vec::new();
    for (i, (&a, &e)) in bytes_a.iter().zip(bytes_b.iter()).enumerate() {
        if a != e {
            let diff = a ^ e;
            for bit in 0..8 {
                if (diff >> bit) & 1 == 1 {
                    let bit_offset = i * 8 + bit;
                    if !allowed_bits.contains(&bit_offset) {
                        violations.push(BitViolation {
                            abs_bit: bit_offset,
                            byte_offset: i,
                            bit_in_byte: bit,
                            val_a: (a >> bit) & 1,
                            val_b: (e >> bit) & 1,
                        });
                    }
                }
            }
        }
    }

    if is_json {
        let report = DsaReport {
            file_a: path_a.clone(),
            file_b: path_b.clone(),
            allowed_bits,
            identical: violations.is_empty(),
            violations,
        };
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("=== Domain Symmetry Auditor (DSA) ===");
        println!("  A: {}", path_a);
        println!("  B: {}", path_b);
        println!("  Allowed bits: {:?}", allowed_bits);
        
        if violations.is_empty() {
            println!("\n[SUCCESS] Bitwise symmetry verified.");
        } else {
            println!("\n[FAILURE] {} unauthorized bit violations found:", violations.len());
            for v in violations.iter().take(20) {
                println!(
                    "  Bit {:>6} (Byte {:>5}, Bit {}): A={} B={}",
                    v.abs_bit, v.byte_offset, v.bit_in_byte, v.val_a, v.val_b
                );
            }
            if violations.len() > 20 {
                println!("  ... and {} more violations", violations.len() - 20);
            }
            process::exit(1);
        }
    }
}
