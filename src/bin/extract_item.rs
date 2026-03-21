use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use d2r_core::item::{HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn main() -> std::io::Result<()> {
    let input_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let bytes = fs::read(input_path).expect("File not found");
    let huffman = HuffmanTree::new();

    let starts = Item::scan_items(&bytes, &huffman);

    for (start, code) in starts {
        if code == "tsc " {
            let item_start_bit = start - 53;
            let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
            let _ = reader.skip(item_start_bit as u32);

            let mut writer = BitWriter::endian(Vec::new(), LittleEndian);
            // Read 128 bits just to be sure we get the whole item (compact items are < 100 bits)
            for _ in 0..128 {
                let bit: bool = reader.read_bit()?;
                writer.write_bit(bit)?;
            }
            let item_data = writer.into_writer();

            fs::create_dir_all("tests/fixtures/items")?;
            fs::write("tests/fixtures/items/portal_scroll.d2i", item_data)?;
            println!("Extracted 'tsc ' to tests/fixtures/items/portal_scroll.d2i");
            break;
        }
    }
    Ok(())
}
