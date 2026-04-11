use std::env;
use std::fs;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

use d2r_core::save::Save;
use d2r_core::item::{Item, HuffmanTree, BitSegment, ItemBitRange};
use d2r_core::verify::args::{ArgParser, ArgSpec};

#[derive(Serialize, Deserialize, Debug)]
struct SbaBaseline {
    fixture: String,
    items: Vec<SbaItem>,
}

#[derive(Serialize, Deserialize, Debug)]
struct SbaItem {
    path: String,
    code: String,
    range: ItemBitRange,
    segments: Vec<BitSegment>,
}

fn main() -> Result<()> {
    let mut parser = ArgParser::new("sba");
    parser.add_spec(ArgSpec::option("fixture", None, Some("fixture"), "Path to the savegame fixture (.d2s)").required());
    parser.add_spec(ArgSpec::option("baseline", None, Some("baseline"), "Path to the JSON baseline file").required());
    parser.add_spec(ArgSpec::flag("generate", None, Some("generate"), "Generate a new baseline from the fixture"));
    parser.add_spec(ArgSpec::flag("verify", None, Some("verify"), "Verify the fixture against an existing baseline"));

    let args: Vec<_> = env::args_os().skip(1).collect();
    let parsed = match parser.parse(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(1);
        }
    };

    let fixture_path = parsed.get("fixture").cloned().unwrap();
    let baseline_path = parsed.get("baseline").cloned().unwrap();
    let is_generate = parsed.is_set("generate");
    let is_verify = parsed.is_set("verify");

    if !is_generate && !is_verify {
        anyhow::bail!("Must specify either --generate or --verify");
    }

    // Enable tracing via environment variable to trigger BitCursor segment recording
    unsafe {
        env::set_var("D2R_ITEM_TRACE", "1");
    }

    let bytes = fs::read(&fixture_path)
        .with_context(|| format!("Failed to read fixture: {}", fixture_path))?;
    
    let save = Save::from_bytes(&bytes)
        .context("Failed to parse save header")?;
    
    let huffman = HuffmanTree::new();
    let is_alpha = save.header.version == 105;
    
    let items = Item::read_player_items(&bytes, &huffman, is_alpha)
        .map_err(|e| anyhow::anyhow!("{}", e))
        .context("Failed to read items")?;

    let mut flattened_items = Vec::new();
    for (i, item) in items.iter().enumerate() {
        flatten_item(item, &i.to_string(), &mut flattened_items);
    }

    let current_baseline = SbaBaseline {
        fixture: fixture_path.clone(),
        items: flattened_items,
    };

    if is_generate {
        let json = serde_json::to_string_pretty(&current_baseline)?;
        fs::write(&baseline_path, json)
            .with_context(|| format!("Failed to write baseline: {}", baseline_path))?;
        println!("Generated baseline for {} items to {}", current_baseline.items.len(), baseline_path);
    } else if is_verify {
        let baseline_json = fs::read_to_string(&baseline_path)
            .with_context(|| format!("Failed to read baseline: {}", baseline_path))?;
        let expected_baseline: SbaBaseline = serde_json::from_str(&baseline_json)?;

        verify_baseline(&expected_baseline, &current_baseline)?;
        println!("Verification successful: 0 segment mismatches across {} items.", current_baseline.items.len());
    }

    Ok(())
}

fn flatten_item(item: &Item, path: &str, result: &mut Vec<SbaItem>) {
    result.push(SbaItem {
        path: path.to_string(),
        code: item.code.clone(),
        range: item.range,
        segments: item.segments.clone(),
    });

    for (i, socketed) in item.socketed_items.iter().enumerate() {
        let sub_path = format!("{}.{}", path, i);
        flatten_item(socketed, &sub_path, result);
    }
}

fn verify_baseline(expected: &SbaBaseline, actual: &SbaBaseline) -> Result<()> {
    if expected.items.len() != actual.items.len() {
        anyhow::bail!(
            "Item count mismatch: expected {}, found {}",
            expected.items.len(),
            actual.items.len()
        );
    }

    for (i, (exp_item, act_item)) in expected.items.iter().zip(actual.items.iter()).enumerate() {
        if exp_item.path != act_item.path {
            anyhow::bail!("Item #{} path mismatch: expected {}, found {}", i, exp_item.path, act_item.path);
        }
        if exp_item.code != act_item.code {
            anyhow::bail!("Item {} code mismatch: expected {}, found {}", exp_item.path, exp_item.code, act_item.code);
        }
        
        if exp_item.segments.len() != act_item.segments.len() {
            anyhow::bail!(
                "Item {} segment count mismatch: expected {}, found {}",
                exp_item.path,
                exp_item.segments.len(),
                act_item.segments.len()
            );
        }

        for (j, (exp_seg, act_seg)) in exp_item.segments.iter().zip(act_item.segments.iter()).enumerate() {
            if exp_seg.label != act_seg.label {
                anyhow::bail!(
                    "Item {} segment #{} label mismatch: expected {}, found {}",
                    exp_item.path, j, exp_seg.label, act_seg.label
                );
            }
            if exp_seg.start != act_seg.start || exp_seg.end != act_seg.end {
                anyhow::bail!(
                    "Item {} segment #{} ({}) bit range mismatch: expected {}-{}, found {}-{}",
                    exp_item.path, j, exp_seg.label, exp_seg.start, exp_seg.end, act_seg.start, act_seg.end
                );
            }
        }
    }

    Ok(())
}
