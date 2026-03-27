use bitstream_io::{BitRead, BitReader, LittleEndian};
use std::fs;
use std::io::Cursor;

fn read_bits<R: BitRead>(reader: &mut R, n: u32) -> u32 {
    let mut value = 0u32;
    for i in 0..n {
        if let Ok(b) = reader.read_bit() {
            if b {
                value |= 1 << i;
            }
        }
    }
    value
}

fn main() {
    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let start_bit = 7790;
    let targets = [310, 14, 31, 1];

    println!("--- Optimized Alpha v105 Property Chain Discovery ---");
    println!("Target Values: {:?}", targets);

    let id_bits = 9;
    let terminator = 511;

    // Use a stack-based search to avoid deep recursion and better control branching
    let mut stack = vec![(start_bit as u64, 0usize, Vec::<(u32, u32, u32)>::new())];
    let mut best_results = Vec::new();
    let mut iterations = 0u64;

    while let Some((bit_pos, depth, sequence)) = stack.pop() {
        iterations += 1;
        if iterations > 1_000_000 {
            break;
        }
        if iterations % 100_000 == 0 {
            print!("."); // Minimal progress indicator
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }

        if depth > 10 {
            continue;
        }

        let byte_offset = bit_pos / 8;
        let bit_offset = bit_pos % 8;
        if byte_offset as usize >= bytes.len() {
            continue;
        }

        let mut reader =
            BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
        for _ in 0..bit_offset {
            let _ = reader.read_bit().ok();
        }

        let id = read_bits(&mut reader, id_bits);
        if id == terminator {
            let values: Vec<u32> = sequence.iter().map(|s| s.2).collect();
            let matches = targets.iter().filter(|t| values.contains(t)).count();
            if matches >= 3 {
                best_results.push((matches, sequence.clone()));
                if matches == 4 {
                    println!(
                        "\n[FOUND PERFECT MATCH at iter {}] Params: {:?}",
                        iterations, sequence
                    );
                }
            }
            continue;
        }

        // Branching logic: prioritize widths that might yield our target values
        // Most stats are 1-12 bits.
        for w in (1..=14).rev() {
            let mut r2 =
                BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
            for _ in 0..bit_offset {
                let _ = r2.read_bit().ok();
            }
            let _id = read_bits(&mut r2, id_bits);
            let val = read_bits(&mut r2, w);

            // PRUNING: If the ID is insane (unlikely for first 10 stats)
            // Or if the value is unreasonably large (> 1024 for most Alpha stats)
            if id > 511 || val > 1024 {
                continue;
            }

            let mut next_seq = sequence.clone();
            next_seq.push((id, w, val));

            stack.push((bit_pos + id_bits as u64 + w as u64, depth + 1, next_seq));
        }
    }

    println!("\nSearch complete in {} iterations.", iterations);
    println!("Top 5 Results:");
    best_results.sort_by(|a, b| b.0.cmp(&a.0));
    for (m, seq) in best_results.iter().take(5) {
        println!("  Score {}: {:?}", m, seq);
    }
}
