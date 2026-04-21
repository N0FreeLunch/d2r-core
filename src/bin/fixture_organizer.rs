use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use d2r_core::verify::args::{ArgParser, ArgSpec};
use d2r_core::save::Save;

fn main() {
    let mut parser = ArgParser::new("fixture-organizer")
        .description("Standardizes and organizes gameplay save fixtures based on internal data");
    
    parser.add_spec(ArgSpec::option("char", Some('c'), Some("char"), "Character name (e.g. TESTDRUID)").required());
    parser.add_spec(ArgSpec::option("quest", Some('q'), Some("quest"), "Quest/Progression context (e.g. Q1)").required());
    parser.add_spec(ArgSpec::option("desc", Some('d'), Some("desc"), "Detailed description (e.g. DonePreAkara)").required());
    parser.add_spec(ArgSpec::positional("src", "Source .d2s file to organize"));

    let parsed = match parser.parse(env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(err) => {
            match err {
                d2r_core::verify::args::ArgError::Help(h) => {
                    println!("{}", h);
                    std::process::exit(0);
                }
                d2r_core::verify::args::ArgError::Error(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    };

    let char_input = parsed.get("char").unwrap();
    let quest_input = parsed.get("quest").unwrap();
    let desc_input = parsed.get("desc").unwrap();
    let src_path_str = parsed.get("src").unwrap();
    let src_path = Path::new(src_path_str);

    if !src_path.exists() {
        eprintln!("Error: Source file does not exist: {}", src_path_str);
        std::process::exit(1);
    }

    let bytes = fs::read(src_path).expect("Failed to read source file");
    
    // Basic validation
    if let Err(e) = Save::from_bytes(&bytes) {
        eprintln!("Error: Not a valid D2R save file: {}", e);
        std::process::exit(1);
    }

    // Intelligence Anchor (User Provided)
    // 161 (0xA1): Normal, 162 (0xA2): NM, 163 (0xA3): Hell
    // Values: 0x01-0x05 (Act 1-5), 0x80+ (Cleared)
    
    let active_act = bytes.get(21).copied().unwrap_or(0);
    let progress_flag = bytes.get(108).copied().unwrap_or(0);
    let normal_prog = bytes.get(161).copied().unwrap_or(0);
    let nm_prog = bytes.get(162).copied().unwrap_or(0);
    let hell_prog = bytes.get(163).copied().unwrap_or(0);
    
    println!("Source Analysis (Alpha v105 Focus):");
    println!("  Offset 021 (active_act):    0x{:02X} (Act {})", active_act, (active_act & 0x7) + 1);
    println!("  Offset 108 (progress_flag): 0x{:02X}", progress_flag);
    println!("  Offset 161 (Normal Prog):   0x{:02X}", normal_prog);
    println!("  Offset 162 (NM Prog):       0x{:02X}", nm_prog);
    println!("  Offset 163 (Hell Prog):     0x{:02X}", hell_prog);
    
    // Alpha v105 Difficulty Logic Refinement (Verified via Discussion 0156)
    // Offset 21 (active_act) bits 3-4 indicate difficulty in v105.
    // 0x00: Normal, 0x08: Nightmare (Bit 3), 0x10: Hell (Bit 4)
    let diff_str = match (active_act & 0x18) >> 3 {
        1 => "nightmare",
        2 => "hell",
        _ => "normal",
    };

    let act_num = (active_act & 0x7) + 1;
    let act_str = format!("act{}", act_num);

    let new_filename = format!("{}_{}_{}.d2s", char_input, quest_input, desc_input);
    
    // Resolve target root
    let root = env::var("D2R_CORE_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| env::current_dir().unwrap());
    
    let target_dir = root.join("tests/fixtures/savegames/gameplay").join(diff_str).join(&act_str);
    let target_path = target_dir.join(&new_filename);

    println!("\nProposed Placement:");
    println!("  Target Directory:  {}", target_dir.display());
    println!("  Target Filename:   {}", new_filename);
    
    if !target_dir.exists() {
        println!("  Creating directory...");
        fs::create_dir_all(&target_dir).expect("Failed to create target directory");
    }

    fs::copy(src_path, &target_path).expect("Failed to copy file to target");
    println!("\nSuccess: File organized to {}", target_path.display());
}
