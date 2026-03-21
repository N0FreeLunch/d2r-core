use std::fs;
use std::io::{self, Cursor};
use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::data::stat_costs::STAT_COSTS;

#[derive(Debug, Clone)]
struct StatRead {
    id: u32,
    name: String,
    value: u32,
}

#[derive(Debug, Clone)]
struct ProbePath {
    start_bit: u64,
    id_bits: u32,
    stats: Vec<StatRead>,
    score: i32,
    terminated: bool,
}

fn read_bits<R: BitRead>(reader: &mut R, n: u32) -> io::Result<u32> {
    let mut value = 0u32;
    for i in 0..n {
        if reader.read_bit()? {
            value |= 1 << i;
        }
    }
    Ok(value)
}

fn is_value_suspicious(id: u32, val: u32) -> bool {
    match id {
        92 => val > 110,
        12 => val > 511,
        7 => val > 1000,
        91 => val > 2000,
        16 | 25 | 31 => val > 1000,
        _ => false,
    }
}

fn get_signature_bonus(id: u32) -> i32 {
    match id {
        16 | 17 | 25 | 31 | 105 | 111 | 135 | 136 | 89 | 83 | 8 => 60,
        _ => 0,
    }
}

fn explore_path(
    bytes: &[u8],
    start_bit: u64,
    id_bits: u32,
    max_depth: usize,
) -> io::Result<ProbePath> {
    let start_byte = (start_bit / 8) as usize;
    let bit_offset = (start_bit % 8) as u32;

    if start_byte + 4 >= bytes.len() {
        return Ok(ProbePath { start_bit, id_bits, stats: Vec::new(), score: 0, terminated: false });
    }

    let mut reader = BitReader::endian(Cursor::new(&bytes[start_byte..]), LittleEndian);
    for _ in 0..bit_offset {
        let _ = reader.read_bit()?;
    }

    let mut path = ProbePath {
        start_bit,
        id_bits,
        stats: Vec::new(),
        score: 0,
        terminated: false,
    };

    for _ in 0..max_depth {
        let Ok(id) = read_bits(&mut reader, id_bits) else { break; };
        
        let terminator = (1 << id_bits) - 1;
        if id == terminator {
            path.score += 100;
            path.terminated = true;
            break;
        }

        let maybe_cost = STAT_COSTS.iter().find(|s| s.id == id);
        if let Some(cost) = maybe_cost {
            path.score += 30;
            path.score += get_signature_bonus(id);
            
            let val = if cost.save_bits > 0 {
                read_bits(&mut reader, cost.save_bits as u32).unwrap_or(0)
            } else {
                0
            };

            if is_value_suspicious(id, val) {
                path.score -= 80;
            }

            path.stats.push(StatRead {
                id,
                name: cost.name.to_string(),
                value: val,
            });
        } else {
            break;
        }
    }

    Ok(path)
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: d2item_bit_width_probe <save_file> debug_mana");
        println!("Usage: d2item_bit_width_probe <save_file> <search_id> [id_bits]");
        println!("Usage: d2item_bit_width_probe <save_file> <base_bit> [window_bits] [max_depth]");
        return Ok(());
    }

    let save_file = &args[1];
    let bytes = fs::read(save_file)?;

    if args[2] == "debug_mana" {
        if let Some(cost) = STAT_COSTS.iter().find(|s| s.id == 8) {
            println!("ID 8 (Mana): save_bits = {}", cost.save_bits);
        }
        if let Some(cost) = STAT_COSTS.iter().find(|s| s.id == 9) {
            println!("ID 9 (Max Mana): save_bits = {}", cost.save_bits);
        }
        return Ok(());
    }

    // Try parsing as search_id if it's small, otherwise interpret as base_bit
    let val2: u64 = args[2].parse().expect("Invalid numeric argument");
    
    if val2 < 1024 {
        let search_id = val2 as u32;
        let id_bits: u32 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(9);
        println!("Searching for ID {} with {} bits...", search_id, id_bits);
        
        for bit in 7000..(bytes.len() as u64 * 8) {
            let start_byte = (bit / 8) as usize;
            let bit_offset = (bit % 8) as u32;
            if start_byte + 4 >= bytes.len() { break; }

            let mut reader = BitReader::endian(Cursor::new(&bytes[start_byte..]), LittleEndian);
            for _ in 0..bit_offset {
                let _ = reader.read_bit()?;
            }
            
            let id = read_bits(&mut reader, id_bits)?;
            if id == search_id {
                let path = explore_path(&bytes, bit, id_bits, 10)?;
                if path.score > 150 {
                    println!("Potential match at bit {}: Score {}, Stats {:?}", bit, path.score, path.stats.iter().map(|s| &s.name).collect::<Vec<_>>());
                }
            }
        }
    } else {
        let base_bit = val2;
        let window: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(16);
        let max_depth: usize = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(10);

        println!("=== D2R Item Bit Width Probe Tool ===");
        println!("Target File: {}", save_file);
        println!("Base Bit: {}, Window: +/-, Max Depth: {}\n", base_bit, window);

        let mut paths = Vec::new();

        for id_bits in [9, 10, 11] {
            let start_range = if base_bit > window { base_bit - window } else { 0 };
            let end_range = base_bit + window;

            for bit in start_range..=end_range {
                let path = explore_path(&bytes, bit, id_bits, max_depth)?;
                if !path.stats.is_empty() {
                    paths.push(path);
                }
            }
        }

        paths.sort_by(|a, b| {
            b.score.cmp(&a.score)
                .then_with(|| b.stats.len().cmp(&a.stats.len()))
        });

        println!("{:<5} | {:<7} | {:<7} | {:<7} | {:<10} | Stats", "Rank", "Score", "ID Bits", "Offset", "Term?");
        println!("{:-<5}-|-{:-<7}-|-{:-<7}-|-{:-<7}-|-{:-<10}-|-------", "", "", "", "", "");

        for (i, path) in paths.iter().take(20).enumerate() {
            let stat_ids: Vec<String> = path.stats.iter().map(|s| format!("{} ({})", s.id, s.name)).collect();
            println!(
                "{:<5} | {:<7} | {:<7} | {:<7} | {:<10} | [{}]",
                i + 1,
                path.score,
                path.id_bits,
                path.start_bit,
                if path.terminated { "YES" } else { "NO" },
                stat_ids.join(", ")
            );
        }
    }

    Ok(())
}
