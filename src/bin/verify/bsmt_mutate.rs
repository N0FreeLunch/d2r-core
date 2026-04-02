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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: bsmt_mutate <save_file.d2s> <target> [--mode absolute|logical]");
        eprintln!("Example (Absolute): bsmt_mutate player.d2s 2392");
        eprintln!("Example (Logical): bsmt_mutate player.d2s Header.Checksum --mode logical");
        process::exit(1);
    }

    let input_path = Path::new(&args[1]);
    let target = &args[2];

    let mut mode = MutationMode::Absolute;
    if let Some(mode_idx) = args.iter().position(|r| r == "--mode") {
        if let Some(mode_val) = args.get(mode_idx + 1) {
            match mode_val.as_str() {
                "absolute" => mode = MutationMode::Absolute,
                "logical" => mode = MutationMode::Logical,
                _ => {
                    eprintln!("Error: Unknown mode '{}'. Use 'absolute' or 'logical'.", mode_val);
                    process::exit(1);
                }
            }
        }
    }

    let bit_offset: usize = match mode {
        MutationMode::Absolute => {
            match target.parse() {
                Ok(val) => val,
                Err(_) => {
                    eprintln!("Error: bit_offset must be a positive integer in absolute mode.");
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

    // 3. Validation and Save
    if let Err(e) = Save::from_bytes(&mutated_bytes) {
        eprintln!("Warning: Mutated bytes failed basic save validation: {}", e);
    }

    if let Err(e) = fs::write(&dest_path, &mutated_bytes) {
        eprintln!("Error writing mutated file: {}", e);
        process::exit(1);
    }

    println!("Finalized and saved to: {:?}", dest_path);
}
