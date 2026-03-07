use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn main() -> std::io::Result<()> {
    let save_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let bytes = fs::read(save_path).expect("Save file not found");
    let huffman = HuffmanTree::new();

    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    reader.skip(903 * 8)?; // Start of JM

    let marker: u16 = reader.read::<16, u16>()?;
    let count: u16 = reader.read::<16, u16>()?;
    println!("Marker: 0x{:04X}, Count: {}", marker, count);

    for i in 0..count {
        let item = Item::from_reader(&mut reader, &huffman)?;
        println!(
            "  Item {}: {}, Page={}, Pos=({}, {})",
            i, item.code, item.page, item.x, item.y
        );
    }

    let end_bit_pos = reader.position_in_bits().unwrap();
    println!("Item list ends at bit pos: {}", end_bit_pos);

    Ok(())
}
