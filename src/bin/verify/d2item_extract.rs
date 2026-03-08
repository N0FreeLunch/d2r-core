use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::env;
use std::fs;
use std::io::Cursor;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: d2item_extract <input.d2s> <item_index> <output.d2i>");
        eprintln!("  input.d2s   Source save file");
        eprintln!("  item_index  The index of the item to extract (0-based)");
        eprintln!("  output.d2i  Output path for the extracted item bits");
        process::exit(1);
    }

    let input_path = &args[1];
    let target_index: usize = args[2].parse().unwrap_or_else(|_| {
        eprintln!("[ERROR] item_index must be a non-negative integer");
        process::exit(1);
    });
    let output_path = &args[3];

    println!("=== d2item_extract ===");
    println!("  Input:  {}", input_path);
    println!("  Index:  {}", target_index);
    println!("  Output: {}", output_path);
    println!();

    // Load the save file
    let bytes = fs::read(input_path).unwrap_or_else(|e| {
        eprintln!("[ERROR] Cannot read '{}': {}", input_path, e);
        process::exit(1);
    });

    // Find first JM
    let jm_pos =
        (0..bytes.len().saturating_sub(1)).find(|&i| bytes[i] == b'J' && bytes[i + 1] == b'M');

    let jm = match jm_pos {
        Some(p) => p,
        None => {
            eprintln!("[ERROR] No JM marker found in '{}'", input_path);
            process::exit(1);
        }
    };

    let item_count = u16::from_le_bytes([bytes[jm + 2], bytes[jm + 3]]);
    if target_index >= item_count as usize {
        eprintln!(
            "[ERROR] item_index {} is out of range. File has {} items.",
            target_index, item_count
        );
        process::exit(1);
    }

    let huffman = HuffmanTree::new();
    let mut reader = BitReader::endian(Cursor::new(&bytes[jm..]), LittleEndian);

    // Skip JM + count (4 bytes = 32 bits)
    let _: u32 = reader.read::<32, u32>().unwrap();

    let mut found_item_bits: Option<Vec<bool>> = None;

    for i in 0..item_count {
        let bit_start = reader.position_in_bits().unwrap_or(0);
        match Item::from_reader(&mut reader, &huffman) {
            Ok(item) => {
                let bit_end = reader.position_in_bits().unwrap_or(0);
                if i as usize == target_index {
                    println!(
                        "  Found item: '{}' ({} bits)",
                        item.code,
                        bit_end - bit_start
                    );

                    // 추출할 비트 범위 계산.
                    // Item::from_reader가 성공했다면, jm + 4바이트(32비트) 오프셋에서의 상대적 위치임.
                    // 실제 데이터에서 정확한 비트열을 가져오기 위해 reader 내부의 비트들을 직접 사용하거나,
                    // Item 구조체의 bits 필드를 사용할 수 있음.
                    // 하지만 실제 원본 비트(패딩 포함 가능성)를 그대로 가져오기 위해 비트 벡터를 구성함.
                    found_item_bits = Some(item.bits.clone());
                    break;
                }
            }
            Err(e) => {
                eprintln!("[ERROR] Failed to parse item {}: {}", i, e);
                process::exit(1);
            }
        }
    }

    let bits = found_item_bits.unwrap_or_else(|| {
        eprintln!(
            "[ERROR] Item at index {} not found during parsing",
            target_index
        );
        process::exit(1);
    });

    // .d2i 파일은 바이트 단위로 저장되므로 비트들을 바이트로 변환.
    // 0018 문서의 해결 방안(Section 3-C)에 따라, 게임 생성 아이템은 보통 바이트 경계에 맞게 저장됨(72비트 등).
    let mut writer = BitWriter::endian(Vec::new(), LittleEndian);
    for bit in bits {
        writer.write_bit(bit).unwrap();
    }
    writer.byte_align().unwrap(); // 바이트 정렬 강제

    let result_bytes = writer.into_writer();

    fs::write(output_path, &result_bytes).unwrap_or_else(|e| {
        eprintln!("[ERROR] Cannot write to '{}': {}", output_path, e);
        process::exit(1);
    });

    println!();
    println!(
        "[OK] Extracted item to {}. Final size: {} bytes ({} bits)",
        output_path,
        result_bytes.len(),
        result_bytes.len() * 8
    );
}
