use std::env;
use std::fs;
use serde::Serialize;
use anyhow::{Result, Context};

use d2r_core::save::{Save, map_core_sections, AttributeSection, class_name};
use d2r_core::item::{Item, HuffmanTree};

#[derive(Serialize)]
struct DiffResult {
    section: String,
    field: String,
    old_value: String,
    new_value: String,
}

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        print_help();
        return Ok(());
    }

    let file1_path = &args[1];
    let file2_path = &args[2];
    let use_json = args.contains(&"--json".to_string());
    let diff_stats = args.contains(&"--stats".to_string()) || (!args.contains(&"--items".to_string()) && args.iter().filter(|a| !a.starts_with("--")).count() >= 3);
    let diff_items = args.contains(&"--items".to_string());

    let bytes1 = fs::read(file1_path).with_context(|| format!("Failed to read {}", file1_path))?;
    let bytes2 = fs::read(file2_path).with_context(|| format!("Failed to read {}", file2_path))?;

    let save1 = Save::from_bytes(&bytes1).context("Failed to parse file 1 header")?;
    let save2 = Save::from_bytes(&bytes2).context("Failed to parse file 2 header")?;

    let mut results = Vec::new();

    // Default to stats if nothing specified
    let actual_diff_stats = diff_stats || (!diff_items);

    if actual_diff_stats {
        // 1. Header Diff
        if save1.header.char_name != save2.header.char_name {
            results.push(DiffResult {
                section: "Header".to_string(),
                field: "Name".to_string(),
                old_value: save1.header.char_name.clone(),
                new_value: save2.header.char_name.clone(),
            });
        }
        if save1.header.char_class != save2.header.char_class {
            results.push(DiffResult {
                section: "Header".to_string(),
                field: "Class".to_string(),
                old_value: class_name(save1.header.char_class).to_string(),
                new_value: class_name(save2.header.char_class).to_string(),
            });
        }
        if save1.header.char_level != save2.header.char_level {
            results.push(DiffResult {
                section: "Header".to_string(),
                field: "Level".to_string(),
                old_value: save1.header.char_level.to_string(),
                new_value: save2.header.char_level.to_string(),
            });
        }

        // 2. Attribute Section Diff
        let map1 = map_core_sections(&bytes1).context("Failed to map sections for file 1")?;
        let map2 = map_core_sections(&bytes2).context("Failed to map sections for file 2")?;
        let attr1 = AttributeSection::parse(&bytes1, map1.gf_pos, map1.if_pos).context("Failed to parse attributes for file 1")?;
        let attr2 = AttributeSection::parse(&bytes2, map2.gf_pos, map2.if_pos).context("Failed to parse attributes for file 2")?;

        let is_alpha1 = save1.header.version == 105;
        let is_alpha2 = save2.header.version == 105;

        // Common stats to check
        let stat_ids = vec![
            (0, "Strength"), (1, "Energy"), (2, "Dexterity"), (3, "Vitality"),
            (4, "StatPoints"), (5, "SkillPoints"), (6, "Life"), (7, "MaxLife"),
            (8, "Mana"), (9, "MaxMana"), (10, "Stamina"), (11, "MaxStamina"),
            (12, "Level"), (13, "Experience"), (14, "Gold"), (15, "StashGold"),
        ];

        for (id, name) in stat_ids {
            let val1 = attr1.actual_value(id, is_alpha1);
            let val2 = attr2.actual_value(id, is_alpha2);

            if val1 != val2 {
                results.push(DiffResult {
                    section: "Attributes".to_string(),
                    field: name.to_string(),
                    old_value: val1.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
                    new_value: val2.map(|v| v.to_string()).unwrap_or_else(|| "N/A".to_string()),
                });
            }
        }
    }

    if diff_items {
        let huffman = HuffmanTree::new();
        let items1 = Item::read_player_items(&bytes1, &huffman, save1.header.version == 105)
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to read items for file 1")?;
        let items2 = Item::read_player_items(&bytes2, &huffman, save2.header.version == 105)
            .map_err(|e| anyhow::anyhow!("{}", e))
            .context("Failed to read items for file 2")?;

        for (i, item1) in items1.iter().enumerate() {
            if let Some(item2) = items2.get(i) {
                if item1.code != item2.code {
                    results.push(DiffResult {
                        section: "Items".to_string(),
                        field: format!("Item #{}", i),
                        old_value: item1.code.clone(),
                        new_value: item2.code.clone(),
                    });
                } else if item1.x != item2.x || item1.y != item2.y || item1.page != item2.page {
                    results.push(DiffResult {
                        section: "Items".to_string(),
                        field: format!("{} #{}", item1.code, i),
                        old_value: format!("Pos({},{},{})", item1.page, item1.x, item1.y),
                        new_value: format!("Pos({},{},{})", item2.page, item2.x, item2.y),
                    });
                }
            } else {
                results.push(DiffResult {
                    section: "Items".to_string(),
                    field: format!("Item #{}", i),
                    old_value: item1.code.clone(),
                    new_value: "REMOVED".to_string(),
                });
            }
        }
        if items2.len() > items1.len() {
            for i in items1.len()..items2.len() {
                results.push(DiffResult {
                    section: "Items".to_string(),
                    field: format!("Item #{}", i),
                    old_value: "NEW".to_string(),
                    new_value: items2[i].code.clone(),
                });
            }
        }
    }

    if use_json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        println!("SSA - Semantic Save-Game Auditor");
        println!("File A: {}", file1_path);
        println!("File B: {}", file2_path);
        println!();

        if results.is_empty() {
            println!("No semantic differences found.");
        } else {
            println!("+------------+----------------------+----------------------+----------------------+");
            println!("| {:<10} | {:<20} | {:<20} | {:<20} |", "Section", "Field", "Value A", "Value B");
            println!("+------------+----------------------+----------------------+----------------------+");
            for res in results {
                println!("| {:<10} | {:<20} | {:<20} | {:<20} |", res.section, res.field, res.old_value, res.new_value);
            }
            println!("+------------+----------------------+----------------------+----------------------+");
        }
    }

    Ok(())
}

fn print_help() {
    println!("Usage: d2save_ssa <file1.d2s> <file2.d2s> [options]");
    println!();
    println!("Options:");
    println!("  --stats    Compare character stats (Level, Exp, Gold, etc.) [Default]");
    println!("  --items    Compare player items (Identity and position)");
    println!("  --json     Output in JSON format");
}
