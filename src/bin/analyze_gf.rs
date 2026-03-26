use d2r_core::save::{map_core_sections, AttributeSection};
use std::fs;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: analyze_gf <file.d2s>");
        return;
    }
    let path = &args[1];
    let bytes = fs::read(path).expect("Failed to read file");
    let map = map_core_sections(&bytes).expect("Failed to map sections");
    
    println!("gf marker at: {}", map.gf_pos);
    println!("if marker at: {}", map.if_pos);
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or([0; 4]));
    
    match AttributeSection::parse(&bytes, &map) {
        Ok(attr) => {
            println!("Attribute Entries: {}", attr.entries.len());
            for entry in &attr.entries {
                let val = attr.actual_value(entry.stat_id, version == 105);
                println!("  ID: {:>3}, Param: {:>3}, Raw: {:>10}, Logical Value: {:?}", 
                    entry.stat_id, 
                    entry.param, 
                    entry.raw_value,
                    val
                );
                if let Some(ref bits) = entry.opaque_bits {
                    println!("    OPAQUE BITS (len={}): {:?}", bits.len(), bits);
                }
            }
        },
        Err(e) => println!("Error parsing attributes: {}", e),
    }
}
