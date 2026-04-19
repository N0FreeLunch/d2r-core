use std::fs;
use anyhow::{bail, Context};
use d2r_core::save::map_core_sections;
use d2r_core::verify::args::{ArgParser, ArgSpec};

fn main() -> anyhow::Result<()> {
    let mut parser = ArgParser::new("d2save_mutate");
    parser.add_spec(ArgSpec::positional("input", "Input save file (.d2s)"));
    parser.add_spec(ArgSpec::option("output", Some('o'), Some("output"), "Output save file (.d2s)").required());
    parser.add_spec(ArgSpec::option("shift-marker", None, Some("shift-marker"), "Shift marker <NAME> <OFFSET>").value_count(2));
    parser.add_spec(ArgSpec::option("delete-marker", None, Some("delete-marker"), "Delete marker <NAME>").value_count(1));

    let parsed = match parser.parse(std::env::args_os().skip(1).collect()) {
        Ok(p) => p,
        Err(d2r_core::verify::args::ArgError::Help(h)) => {
            println!("{}", h);
            return Ok(());
        }
        Err(d2r_core::verify::args::ArgError::Error(e)) => {
            bail!("error: {}\n\n{}", e, parser.usage());
        }
    };

    let input_path = parsed.get("input").unwrap();
    let output_path = parsed.get("output").unwrap();

    let mut bytes = fs::read(input_path).context("Failed to read input file")?;
    
    // Validate version 105
    if bytes.len() < 8 {
        bail!("Input file is too small");
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != 105 {
        bail!("Only Alpha v105 is supported. Found version {}", version);
    }

    let map = map_core_sections(&bytes).context("Failed to map core sections")?;

    if let Some(shift_args) = parsed.get_vec("shift-marker") {
        let name = &shift_args[0];
        let offset: isize = shift_args[1].parse().context("Invalid shift offset")?;
        mutate_marker(&mut bytes, &map, name, Some(offset))?;
    } else if let Some(delete_args) = parsed.get_vec("delete-marker") {
        let name = &delete_args[0];
        mutate_marker(&mut bytes, &map, name, None)?;
    } else {
        bail!("No mutation operation specified. Use --shift-marker or --delete-marker.");
    }

    fs::write(output_path, &bytes).context("Failed to write output file")?;
    println!("Successfully mutated save and saved to {}", output_path);

    Ok(())
}

fn mutate_marker(bytes: &mut [u8], map: &d2r_core::save::SaveSectionMap, name: &str, shift: Option<isize>) -> anyhow::Result<()> {
    let (pos, marker_bytes) = match name {
        "Woo!" => (map.woo_pos, b"Woo!".as_slice()),
        "WS" => (map.ws_pos, b"WS".as_slice()),
        "w4" => (map.w4_pos, b"w4".as_slice()),
        _ => bail!("Unknown marker name: {}. Supported: Woo!, WS, w4", name),
    };

    let original_pos = pos.ok_or_else(|| anyhow::anyhow!("Marker {} not found in input file", name))?;
    let len = marker_bytes.len();

    println!("Original marker {} at range 0x{:X}..0x{:X}", name, original_pos, original_pos + len);

    // Zero out original
    for i in 0..len {
        bytes[original_pos + i] = 0;
    }

    if let Some(s) = shift {
        let new_pos_i = original_pos as isize + s;
        if new_pos_i < 0 {
            bail!("Shifted marker {} out of bounds (negative offset)", name);
        }
        let new_pos = new_pos_i as usize;
        if new_pos + len > bytes.len() {
            bail!("Shifted marker {} out of bounds (beyond EOF)", name);
        }
        
        for i in 0..len {
            bytes[new_pos + i] = marker_bytes[i];
        }
        println!("Shifted marker {} to range 0x{:X}..0x{:X}", name, new_pos, new_pos + len);
    } else {
        println!("Deleted marker {} (zero-filled)", name);
    }

    println!("Note: Checksum was NOT recalculated.");
    Ok(())
}