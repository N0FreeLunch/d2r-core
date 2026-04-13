// Copyright 2026 N0FreeLunch (https://github.com/N0FreeLunch/d2r-core)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::env;
use std::fs;
use std::path::Path;
use std::process;
use d2r_core::verify::mutation::{mutate_absolute_bit_and_finalize, MutationMode, resolve_logical_address};      
use d2r_core::save::Save;
use d2r_core::item::{Item, HuffmanTree};
use d2r_core::verify::args::{ArgParser, ArgSpec, ArgError};

fn main() {
    let mut parser = ArgParser::new("bsmt_mutate")
        .description("Safely mutates D2R save files in an isolated environment (tmp/forensics/) for testing and exploration");

    parser.add_spec(ArgSpec::positional("save_file", "path to the D2R save file (.d2s)"));
    parser.add_spec(ArgSpec::positional("target", "bit offset (absolute) or logical address (e.g. Header.Checksum)"));
    parser.add_spec(ArgSpec::option("mode", Some('m'), Some("mode"), "mutation mode: 'absolute' (default) or 'logical'").with_default("absolute"));

    let args: Vec<_> = env::args_os().skip(1).collect();
    let parsed = match parser.parse(args) {
        Ok(p) => p,
        Err(ArgError::Help(h)) => {
            println!("{}", h);
            process::exit(0);
        }
        Err(ArgError::Error(e)) => {
            eprintln!("Error: {}\n\n{}", e, parser.usage());
            process::exit(1);
        }
    };

    let input_path = Path::new(parsed.get("save_file").unwrap());
    let target = parsed.get("target").unwrap();
    let mode_str = parsed.get("mode").unwrap();

    let mode = match mode_str.as_str() {
        "absolute" => MutationMode::Absolute,
        "logical" => MutationMode::Logical,
        _ => {
            eprintln!("Error: Unknown mode '{}'. Use 'absolute' or 'logical'.", mode_str);
            process::exit(1);
        }
    };

    let bit_offset: usize = match mode {
        MutationMode::Absolute => {
            match target.parse() {
                Ok(val) => val,
                Err(_) => {
                    eprintln!("Error: target must be a positive integer in absolute mode.");
                    process::exit(1);
                }
            }
        }
        MutationMode::Logical => {
            match resolve_logical_address(target) {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("Error resolving logical address: {}", e);
                    process::exit(1);
                }
            }
        }
    };

    if !input_path.exists() {
        eprintln!("Error: Input file '{:?}' does not exist.", input_path);
        process::exit(1);
    }

    // 1. Isolation: Create tmp/forensics/ if it doesn't exist
    let forensics_dir = Path::new("tmp/forensics");
    if let Err(e) = fs::create_dir_all(forensics_dir) {
        eprintln!("Error: Could not create forensics directory: {}", e);
        process::exit(1);
    }

    let file_stem = input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");
    let dest_filename = format!("{}_mutated_{}.d2s", file_stem, bit_offset);
    let dest_path = forensics_dir.join(dest_filename);

    println!("Isolated to: {:?}", dest_path);

    // 2. Mutation
    let original_bytes = match fs::read(input_path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error reading input file: {}", e);
            process::exit(1);
        }
    };

    println!("Mutation applied at bit offset: {}", bit_offset);
    let mutated_bytes = match mutate_absolute_bit_and_finalize(&original_bytes, bit_offset) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("Error during mutation/finalization: {}", e);
            process::exit(1);
        }
    };

    // 3. Validation and Diagnostic Reporting
    let mut diagnostic_failed = false;
    let mut report = String::new();

    match Save::from_bytes(&mutated_bytes) {
        Ok(save) => {
            let huffman = HuffmanTree::new();
            // Alpha v105 detection
            let is_alpha = save.header.version == 6 || save.header.version == 105;

            match Item::read_player_items(&mutated_bytes, &huffman, is_alpha) {
                Ok(items) => {
                    // Find JM #0 to get the expected count from the header
                    if let Some(jm_pos) = mutated_bytes.windows(2).position(|w| w == b"JM") {
                        let expected_count = u16::from_le_bytes([mutated_bytes[jm_pos + 2], mutated_bytes[jm_pos + 3]]) as usize;

                        if items.len() != expected_count {
                            diagnostic_failed = true;
                            report = format!(
                                "[Bit {}] [Rel +16] [read_player_items] Item count mismatch: expected {}, found {}",
                                jm_pos * 8 + 16,
                                expected_count,
                                items.len()
                            );
                        }
                    }
                }
                Err(e) => {
                    diagnostic_failed = true;
                    report = format!("{}", e);
                }
            }
        }
        Err(e) => {
            diagnostic_failed = true;
            report = format!("Basic save validation failed: {}", e);
        }
    }

    if diagnostic_failed {
        println!("\n--- Diagnostic Report ---");
        println!("{}", report);
        println!("-------------------------\n");
    } else {
        println!("Parse check: ok");
    }

    if let Err(e) = fs::write(&dest_path, &mutated_bytes) {
        eprintln!("Error writing mutated file: {}", e);
        process::exit(1);
    }

    println!("Finalized and saved to: {:?}", dest_path);
}
