use bitstream_io::{BitRead, BitReader as IoBitReader, LittleEndian};
use std::env;
use std::fs;
use std::io::Cursor;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: d2item_v5_peek <file> <offset_bits> <count>");
        return;
    }
    let path = &args[1];
    let offset = args[2].parse::<u64>().expect("invalid offset");
    let count = args[3].parse::<u32>().expect("invalid count");
    let bytes = fs::read(path).expect("failed to read file");

    let mut reader = IoBitReader::endian(Cursor::new(&bytes), LittleEndian);
    reader.skip(offset as u32).expect("failed to skip");

    println!("Bit Dump at offset {}:", offset);
    println!("Index | Bit | Pos | Octet View");
    println!("------|-----|-----|-----------");
    for i in 0..count {
        let bit = reader.read_bit().expect("failed to read bit");
        let current_bit = offset + i as u64;
        let bit_in_octet = i % 8;
        
        if i % 8 == 0 {
            print!("{:>5} | ", i);
        }
        
        print!("{}", if bit { "1" } else { "0" });

        if i % 8 == 7 {
            println!(" | {:>3} | bits {}-{}", current_bit, current_bit - 7, current_bit);
        } else if i % 4 == 3 {
            print!(" ");
        }
    }
    println!();
}
