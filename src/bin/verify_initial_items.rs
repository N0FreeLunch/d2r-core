use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn main() -> std::io::Result<()> {
    let input_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let bytes = fs::read(input_path).expect("File not found");
    let huffman = HuffmanTree::new();

    let starts = Item::scan_items(&bytes, &huffman);
    println!("Found {} items:", starts.len());

    for (start, code) in starts {
        let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
        let _ = reader.skip((start - 53) as u32);

        let flags: u32 = reader.read::<32, u32>()?;
        let version: u16 = reader.read::<3, u16>()?;
        let mode: u8 = reader.read::<3, u8>()?;
        let loc: u8 = reader.read::<4, u8>()?;
        let x: u8 = reader.read::<4, u8>()?;
        let y: u8 = reader.read::<4, u8>()?;
        let page: u8 = reader.read::<3, u8>()?;

        println!(
            "  Item '{}' at bit {}: Flags=0x{:08X}, Mode={}, Loc={}, X={}, Y={}, Page={}",
            code,
            start - 53,
            flags,
            mode,
            loc,
            x,
            y,
            page
        );
    }

    Ok(())
}
