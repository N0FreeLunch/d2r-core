use bitstream_io::{BitRead, BitReader, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn main() -> std::io::Result<()> {
    let save_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let bytes = fs::read(save_path).expect("Save file not found");
    let huffman = HuffmanTree::new();

    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    reader.skip(903 * 8)?;

    let _: u16 = reader.read::<16, u16>()?;
    let count: u16 = reader.read::<16, u16>()?;

    for _ in 0..count {
        let _ = Item::from_reader(&mut reader, &huffman)?;
    }

    let end_bit_pos = reader.position_in_bits().unwrap();
    println!("Item list ends at bit pos: {}", end_bit_pos);

    // Read next 32 bits and see if it's 'JM' + count
    let next_marker: u16 = reader.read::<16, u16>()?;
    let next_count: u16 = reader.read::<16, u16>()?;
    println!(
        "Next Marker: 0x{:04X} ('{}'), Next Count: {}",
        next_marker,
        (next_marker as u8 as char),
        next_count
    );

    Ok(())
}
