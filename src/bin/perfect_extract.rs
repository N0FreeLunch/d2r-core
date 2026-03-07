use bitstream_io::{BitRead, BitReader, LittleEndian};
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
            reader.skip(item_start_bit as u32)?;

            let item = Item::from_reader(&mut reader, &huffman)?;
            println!("Extracted '{}' with {} bits", item.code, item.bits.len());

            // Save as raw bits for .d2i
            let mut writer = bitstream_io::BitWriter::endian(Vec::new(), LittleEndian);
            for bit in item.bits {
                bitstream_io::BitWrite::write_bit(&mut writer, bit)?;
            }
            let item_data = writer.into_writer();

            fs::create_dir_all("tests/fixtures/items")?;
            fs::write("tests/fixtures/items/portal_scroll_perfect.d2i", item_data)?;
            break;
        }
    }
    Ok(())
}
