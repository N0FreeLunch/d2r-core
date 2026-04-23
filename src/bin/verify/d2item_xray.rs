use anyhow::{Context, Result};
use d2r_core::item::{HuffmanTree, Item};
use d2r_core::save::Save;
use d2r_core::verify::args::{ArgParser, ArgSpec};
use std::env;
use std::fs;

fn main() -> Result<()> {
    let mut parser = ArgParser::new("d2item_xray");
    parser.add_spec(ArgSpec::positional("fixture", "Path to the savegame fixture (.d2s)"));
    parser.add_spec(ArgSpec::flag("verbose", Some('v'), Some("verbose"), "Show detailed segment bitstreams"));

    let args: Vec<_> = env::args_os().skip(1).collect();
    use d2r_core::verify::args::ArgError;
    let parsed = match parser.parse(args) {
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

    let fixture_path = parsed.get("fixture").cloned().context("Fixture path required")?;
    let verbose = parsed.is_set("verbose");

    // Force tracing on to collect segments
    unsafe {
        env::set_var("D2R_ITEM_TRACE", "1");
    }

    let bytes = fs::read(&fixture_path)
        .with_context(|| format!("Failed to read fixture: {}", fixture_path))?;
    
    let save = Save::from_bytes(&bytes)
        .context("Failed to parse save header")?;
    
    let huffman = HuffmanTree::new();
    let is_alpha = save.header.version == 105;
    
    println!("=== d2item_xray: {} ===", fixture_path);
    println!("Version: {}, Alpha: {}", save.header.version, is_alpha);

    let items = Item::read_player_items(&bytes, &huffman, is_alpha)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to read items from fixture")?;

    println!("Found {} items in fixture.", items.len());

    for (idx, item) in items.iter().enumerate() {
        println!("\nItem #{} [{}]: bits {}-{} (len={})", 
            idx, item.code, item.range.start, item.range.end, item.range.end - item.range.start);
        
        // Re-serialize to check for symmetry
        let reserialized = item.to_bytes(is_alpha).map_err(|e| anyhow::anyhow!("{}", e))?;
        let original_bits = &item.bits; // item.bits contains the raw bits read

        if original_bits.len() != reserialized.len() * 8 && !is_alpha {
             // In non-alpha, bit-length matters. In alpha, we align to bytes.
        }

        println!("  Segments ({}):", item.segments.len());
        for seg in &item.segments {
            let indent = "  ".repeat(seg.depth + 1);
            let len = seg.end - seg.start;
            print!("{}[{:>4}..{:>4}] (len={:>2}) {:<20}", indent, seg.start, seg.end, len, seg.label);
            
            if verbose {
                // Show bits
                let mut bit_str = String::new();
                for i in seg.start..seg.end {
                    if i < item.bits.len() as usize {
                        bit_str.push(if item.bits[i] { '1' } else { '0' });
                    }
                }
                print!(" | {}", bit_str);
            }
            println!();
        }

        for (c_idx, child) in item.socketed_items.iter().enumerate() {
            println!("    Socketed #{} [{}]: bits {}-{}", c_idx, child.code, child.range.start, child.range.end);
        }
    }

    Ok(())
}
