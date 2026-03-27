use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::data::item_codes::ITEM_TEMPLATES;
use d2r_core::data::stat_costs::STAT_COSTS;
use d2r_core::item::HuffmanTree;
use std::env;
use std::fs;
use std::io::{self, Cursor};

#[derive(Debug)]
struct Field {
    start: usize,
    end: usize,
    label: String,
    value: String,
}

type Reader<'a> = BitReader<Cursor<&'a [u8]>, LittleEndian>;

fn read_bits(reader: &mut Reader<'_>, count: u32) -> io::Result<u32> {
    let mut value = 0;
    for i in 0..count {
        if reader.read_bit()? {
            value |= 1 << i;
        }
    }
    Ok(value)
}

fn read_field(
    reader: &mut Reader<'_>,
    fields: &mut Vec<Field>,
    label: impl Into<String>,
    count: u32,
) -> io::Result<u32> {
    let start = reader.position_in_bits()? as usize;
    let value = read_bits(reader, count)?;
    let end = reader.position_in_bits()? as usize;
    fields.push(Field {
        start,
        end,
        label: label.into(),
        value: format!("{value}"),
    });
    Ok(value)
}

fn read_field_hex(
    reader: &mut Reader<'_>,
    fields: &mut Vec<Field>,
    label: impl Into<String>,
    count: u32,
) -> io::Result<u32> {
    let start = reader.position_in_bits()? as usize;
    let value = read_bits(reader, count)?;
    let end = reader.position_in_bits()? as usize;
    fields.push(Field {
        start,
        end,
        label: label.into(),
        value: format!("0x{value:X}"),
    });
    Ok(value)
}

fn read_player_name(
    reader: &mut Reader<'_>,
    fields: &mut Vec<Field>,
    label: &str,
) -> io::Result<String> {
    let start = reader.position_in_bits()? as usize;
    let mut name = String::new();
    loop {
        let ch = read_bits(reader, 7)? as u8;
        if ch == 0 {
            break;
        }
        name.push(ch as char);
    }
    let end = reader.position_in_bits()? as usize;
    fields.push(Field {
        start,
        end,
        label: label.to_string(),
        value: if name.is_empty() {
            "<empty>".to_string()
        } else {
            name.clone()
        },
    });
    Ok(name)
}

fn read_code(
    reader: &mut Reader<'_>,
    huffman: &HuffmanTree,
    fields: &mut Vec<Field>,
) -> io::Result<String> {
    let start = reader.position_in_bits()? as usize;
    let mut code = String::new();
    for _ in 0..4 {
        code.push(huffman.decode(reader)?);
    }
    let end = reader.position_in_bits()? as usize;
    fields.push(Field {
        start,
        end,
        label: "code".to_string(),
        value: code.clone(),
    });
    Ok(code)
}

fn read_property_list(reader: &mut Reader<'_>, fields: &mut Vec<Field>) -> io::Result<()> {
    loop {
        let start = reader.position_in_bits()? as usize;
        let stat_id = read_bits(reader, 9)?;
        let mut value = format!("id={stat_id}");
        if stat_id == 0x1FF {
            let end = reader.position_in_bits()? as usize;
            fields.push(Field {
                start,
                end,
                label: "property_terminator".to_string(),
                value,
            });
            return Ok(());
        }

        let Some(cost) = STAT_COSTS.iter().find(|entry| entry.id == stat_id) else {
            let end = reader.position_in_bits()? as usize;
            fields.push(Field {
                start,
                end,
                label: "property_unknown".to_string(),
                value,
            });
            return Ok(());
        };

        if cost.save_param_bits > 0 {
            let param = read_bits(reader, cost.save_param_bits as u32)?;
            value.push_str(&format!(", param={param}"));
        }
        if cost.save_bits > 0 {
            let stat_value = read_bits(reader, cost.save_bits as u32)?;
            value.push_str(&format!(", value={stat_value}"));
        }

        let end = reader.position_in_bits()? as usize;
        fields.push(Field {
            start,
            end,
            label: format!("property_{}", cost.name),
            value,
        });
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin d2item_blob_inspect -- <raw_item_file>");
        std::process::exit(1);
    }

    let bytes = fs::read(&args[1])?;
    let huffman = HuffmanTree::new();
    let mut reader = BitReader::endian(Cursor::new(bytes.as_slice()), LittleEndian);
    let mut fields = Vec::new();

    let flags = read_field_hex(&mut reader, &mut fields, "flags", 32)?;
    let _version = read_field(&mut reader, &mut fields, "version", 3)?;
    let _mode = read_field(&mut reader, &mut fields, "mode", 3)?;
    let _location = read_field(&mut reader, &mut fields, "location", 4)?;
    let _x = read_field(&mut reader, &mut fields, "x", 4)?;
    let _y = read_field(&mut reader, &mut fields, "y", 4)?;
    let _page = read_field(&mut reader, &mut fields, "page", 3)?;

    let is_socketed = (flags & (1 << 11)) != 0;
    let is_ear = (flags & (1 << 16)) != 0;
    let is_compact = (flags & (1 << 21)) != 0;
    let is_personalized = (flags & (1 << 24)) != 0;
    let is_runeword = (flags & (1 << 26)) != 0;

    let code = if is_ear {
        let _ = read_field(&mut reader, &mut fields, "ear_class", 3)?;
        let _ = read_field(&mut reader, &mut fields, "ear_level", 7)?;
        let _ = read_player_name(&mut reader, &mut fields, "ear_name")?;
        "ear ".to_string()
    } else {
        read_code(&mut reader, &huffman, &mut fields)?
    };

    if !is_ear {
        let _ = read_field(&mut reader, &mut fields, "post_code_d2r_bits", 3)?;
    }

    if !is_compact && !is_ear {
        let template = ITEM_TEMPLATES
            .iter()
            .find(|entry| entry.code == code.trim());

        let _ = read_field_hex(&mut reader, &mut fields, "item_id", 32)?;
        let _ = read_field(&mut reader, &mut fields, "item_level", 7)?;
        let quality = read_field(&mut reader, &mut fields, "quality", 4)?;

        let has_multiple_graphics =
            read_field(&mut reader, &mut fields, "has_multiple_graphics", 1)? != 0;
        if has_multiple_graphics {
            let _ = read_field(&mut reader, &mut fields, "graphic_id", 3)?;
        }

        let has_class_specific_data =
            read_field(&mut reader, &mut fields, "has_class_specific_data", 1)? != 0;
        if has_class_specific_data {
            let _ = read_field(&mut reader, &mut fields, "class_specific_data", 11)?;
        }

        let quality_block_start = reader.position_in_bits()? as usize;
        match quality {
            1 | 3 => {
                let _ = read_field(&mut reader, &mut fields, "quality_data.low_high_id", 3)?;
            }
            4 => {
                let _ = read_field(&mut reader, &mut fields, "quality_data.magic_prefix", 11)?;
                let _ = read_field(&mut reader, &mut fields, "quality_data.magic_suffix", 11)?;
            }
            5 => {
                let _ = read_field(&mut reader, &mut fields, "quality_data.set_id", 12)?;
            }
            6 | 8 => {
                let _ = read_field(&mut reader, &mut fields, "quality_data.rare_name_1", 8)?;
                let _ = read_field(&mut reader, &mut fields, "quality_data.rare_name_2", 8)?;
                for index in 0..3 {
                    let prefix_present = read_field(
                        &mut reader,
                        &mut fields,
                        format!("quality_data.affix_pair_{}_prefix_present", index + 1),
                        1,
                    )? != 0;
                    if prefix_present {
                        let _ = read_field(
                            &mut reader,
                            &mut fields,
                            format!("quality_data.affix_pair_{}_prefix_id", index + 1),
                            11,
                        )?;
                    }

                    let suffix_present = read_field(
                        &mut reader,
                        &mut fields,
                        format!("quality_data.affix_pair_{}_suffix_present", index + 1),
                        1,
                    )? != 0;
                    if suffix_present {
                        let _ = read_field(
                            &mut reader,
                            &mut fields,
                            format!("quality_data.affix_pair_{}_suffix_id", index + 1),
                            11,
                        )?;
                    }
                }
            }
            7 => {
                let _ = read_field(&mut reader, &mut fields, "quality_data.unique_id", 12)?;
            }
            _ => {}
        }
        let quality_block_end = reader.position_in_bits()? as usize;
        fields.push(Field {
            start: quality_block_start,
            end: quality_block_end,
            label: "quality_block_total".to_string(),
            value: format!("{} bits", quality_block_end - quality_block_start),
        });

        if is_runeword {
            let _ = read_field(&mut reader, &mut fields, "runeword_id", 12)?;
            let _ = read_field(&mut reader, &mut fields, "runeword_param", 4)?;
        }

        if is_personalized {
            let _ = read_player_name(&mut reader, &mut fields, "personalized_name")?;
        }

        if code.trim() == "tbk" || code.trim() == "ibk" {
            let _ = read_field(&mut reader, &mut fields, "book_tome_fill", 5)?;
        }

        let has_realm_data = read_field(&mut reader, &mut fields, "has_realm_data", 1)? != 0;
        if has_realm_data {
            let _ = read_field_hex(&mut reader, &mut fields, "realm_data_1", 32)?;
            let _ = read_field_hex(&mut reader, &mut fields, "realm_data_2", 32)?;
            let _ = read_field_hex(&mut reader, &mut fields, "realm_data_3", 32)?;
        }

        if let Some(template) = template {
            if template.is_armor {
                let _ = read_field(&mut reader, &mut fields, "defense", 11)?;
            }
            if template.has_durability {
                let max_durability = read_field(&mut reader, &mut fields, "max_durability", 8)?;
                if max_durability > 0 {
                    let _ = read_field(&mut reader, &mut fields, "current_durability", 9)?;
                    let _ = read_field(&mut reader, &mut fields, "durability_bonus", 1)?;
                }
            }
            if template.is_stackable {
                let _ = read_field(&mut reader, &mut fields, "quantity", 9)?;
            }
        }

        if is_socketed {
            let _ = read_field(&mut reader, &mut fields, "socket_count", 4)?;
        }

        println!("--- Property Lists ---");
        while reader.position_in_bits()? < (bytes.len() * 8) as u64 {
            let _ = read_property_list(&mut reader, &mut fields);
            if reader.position_in_bits()? % 8 == 0
                && reader.position_in_bits()? == (bytes.len() * 8) as u64
            {
                break;
            }
        }
    }

    let consumed_bits = reader.position_in_bits()? as usize;
    let aligned_bits = consumed_bits.div_ceil(8) * 8;

    println!("bytes={}", bytes.len());
    println!("bits_consumed={consumed_bits}");
    println!("padding_bits={}", aligned_bits - consumed_bits);
    println!();

    for field in fields {
        println!(
            "{:>4}-{:>4} ({:>3}) {:<36} {}",
            field.start,
            field.end,
            field.end - field.start,
            field.label,
            field.value
        );
    }

    Ok(())
}
