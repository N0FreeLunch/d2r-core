use std::env;
use std::fs;
use serde::{Serialize, Deserialize};
use anyhow::{Result, Context};

use d2r_core::save::Save;
use d2r_core::item::{Item, HuffmanTree, BitSegment, ItemBitRange};
use d2r_core::verify::args::{ArgParser, ArgSpec};

use d2r_core::verify::{Report, ReportMetadata, ReportStatus, ReportIssue};

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

#[derive(Serialize)]
struct SbaJsonPayload {
    fixture: String,
    baseline: String,
    mode: String,
    item_count: usize,
    mismatch_count: usize,
}

fn main() -> Result<()> {
    let mut parser = ArgParser::new("sba");
    parser.add_spec(ArgSpec::option("fixture", None, Some("fixture"), "Path to the savegame fixture (.d2s)").required());
    parser.add_spec(ArgSpec::option("baseline", None, Some("baseline"), "Path to the JSON baseline file").required());
    parser.add_spec(ArgSpec::flag("generate", None, Some("generate"), "Generate a new baseline from the fixture"));
    parser.add_spec(ArgSpec::flag("verify", None, Some("verify"), "Verify the fixture against an existing baseline"));
    parser.add_spec(ArgSpec::flag("json", None, Some("json"), "Emit results in shared Report JSON format"));

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
    let is_json = parsed.is_set("json");

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

    let mut issues = Vec::new();

    if is_generate {
        let json = serde_json::to_string_pretty(&current_baseline)?;
        fs::write(&baseline_path, json)
            .with_context(|| format!("Failed to write baseline: {}", baseline_path))?;
        if !is_json {
            println!("Generated baseline for {} items to {}", current_baseline.items.len(), baseline_path);
        }
    } else if is_verify {
        let baseline_json = fs::read_to_string(&baseline_path)
            .with_context(|| format!("Failed to read baseline: {}", baseline_path))?;
        let expected_baseline: SbaBaseline = serde_json::from_str(&baseline_json)?;

        if let Err(e) = verify_baseline(&expected_baseline, &current_baseline, &mut issues) {
            if !is_json {
                return Err(e);
            }
        }
        
        if !is_json {
            println!("Verification successful: 0 segment mismatches across {} items.", current_baseline.items.len());
        }
    }

    if is_json {
        let status = if issues.is_empty() { ReportStatus::Ok } else { ReportStatus::Fail };
        let version = if is_alpha { "105".to_string() } else { format!("0x{:04X}", save.header.version) };
        let metadata = ReportMetadata::new("sba", &fixture_path, &version);
        let payload = SbaJsonPayload {
            fixture: current_baseline.fixture,
            baseline: baseline_path,
            mode: if is_generate { "generate".to_string() } else { "verify".to_string() },
            item_count: current_baseline.items.len(),
            mismatch_count: issues.len(),
        };
        let report = Report::new(metadata, status)
            .with_results(payload)
            .with_issues(issues);

        println!("{}", serde_json::to_string(&report)?);
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

fn verify_baseline(expected: &SbaBaseline, actual: &SbaBaseline, issues: &mut Vec<ReportIssue>) -> Result<()> {
    if expected.items.len() != actual.items.len() {
        let msg = format!(
            "Item count mismatch: expected {}, found {}",
            expected.items.len(),
            actual.items.len()
        );
        issues.push(ReportIssue { kind: "structure".to_string(), message: msg.clone(), bit_offset: None });
        anyhow::bail!(msg);
    }

    for (i, (exp_item, act_item)) in expected.items.iter().zip(actual.items.iter()).enumerate() {
        if exp_item.path != act_item.path {
            let msg = format!("Item #{} path mismatch: expected {}, found {}", i, exp_item.path, act_item.path);
            issues.push(ReportIssue { kind: "structure".to_string(), message: msg.clone(), bit_offset: None });
            anyhow::bail!(msg);
        }
        if exp_item.code != act_item.code {
            let msg = format!("Item {} code mismatch: expected {}, found {}", exp_item.path, exp_item.code, act_item.code);
            issues.push(ReportIssue { kind: "data".to_string(), message: msg.clone(), bit_offset: Some(act_item.range.start) });
            anyhow::bail!(msg);
        }
        
        if exp_item.segments.len() != act_item.segments.len() {
            let msg = format!(
                "Item {} segment count mismatch: expected {}, found {}",
                exp_item.path,
                exp_item.segments.len(),
                act_item.segments.len()
            );
            issues.push(ReportIssue { kind: "structural_segment".to_string(), message: msg.clone(), bit_offset: Some(act_item.range.start) });
            anyhow::bail!(msg);
        }

        for (j, (exp_seg, act_seg)) in exp_item.segments.iter().zip(act_item.segments.iter()).enumerate() {
            if exp_seg.label != act_seg.label {
                let msg = format!(
                    "Item {} segment #{} label mismatch: expected {}, found {}",
                    exp_item.path, j, exp_seg.label, act_seg.label
                );
                issues.push(ReportIssue { kind: "structural_label".to_string(), message: msg.clone(), bit_offset: Some(act_item.range.start + act_seg.start as u64) });
                anyhow::bail!(msg);
            }
            if exp_seg.start != act_seg.start || exp_seg.end != act_seg.end {
                let msg = format!(
                    "Item {} segment #{} ({}) bit range mismatch: expected {}-{}, found {}-{}",
                    exp_item.path, j, exp_seg.label, exp_seg.start, exp_seg.end, act_seg.start, act_seg.end
                );
                issues.push(ReportIssue { kind: "structural_range".to_string(), message: msg.clone(), bit_offset: Some(act_item.range.start + act_seg.start as u64) });
                anyhow::bail!(msg);
            }
        }
    }

    Ok(())
}
