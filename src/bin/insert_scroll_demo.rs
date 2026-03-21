use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use d2r_core::item::{Checksum, HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn main() -> std::io::Result<()> {
    let save_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let bytes = fs::read(save_path).expect("Save file not found");
    let huffman = HuffmanTree::new();

    // We'll use our "perfect" 70-bit scroll as a template
    // Portal Scroll Bits (Compact):
    // Flags=0x00A22010 (Compact bit set)
    // Code='tsc '
    let scroll_template_bits = vec![
        true, false, false, false, true, false, false, false, false, false, false, false, false,
        false, true, false, false, false, false, false, false, true, false, true, false, true,
        false, false, false, false, false, false, // Flags (32)
        true, false, false, // Version (3)
        false, false, false, // Mode (0: Stored)
        false, false, false, false, // Loc (0: Inventory)
        false, false, false, false, // X (Temporary)
        false, false, false, false, // Y (Temporary)
        true, false, false, // Page (1: Inventory)
        // Huffman 'tsc '
        false, true, true, false, false, // 't'
        false, false, true, false, // 's'
        false, true, false, false, false, // 'c'
        true, false, // ' '
        false, // num_sockets (0)
    ];
    // Total 70 bits.

    // 1. Load the save into a bit-vector for easy manipulation
    let mut reader = BitReader::endian(Cursor::new(&bytes), LittleEndian);
    let mut all_bits = Vec::new();
    for _ in 0..(bytes.len() * 8) {
        all_bits.push(reader.read_bit()?);
    }

    // 2. Locate the Item List marker (JM) at 903
    let list_start_bit = 903 * 8;

    // 3. We will REWRITE the item list to have exactly 10 scrolls in a 2x5 grid
    let new_count: u16 = 10;
    let mut new_item_list_bits = Vec::new();

    // Marker JM (0x4D4A)
    for i in 0..16 {
        new_item_list_bits.push((0x4D4Au16 >> i) & 1 != 0);
    }
    // Count (10)
    for i in 0..16 {
        new_item_list_bits.push((new_count >> i) & 1 != 0);
    }

    for i in 0..10 {
        let x = i % 10;
        let y = i / 10; // All in one row (0,0 to 9,0)

        let mut scroll = scroll_template_bits.clone();
        Item::set_bits(&mut scroll, 42, x as u32, 4);
        Item::set_bits(&mut scroll, 46, y as u32, 4);

        new_item_list_bits.extend(scroll);
    }

    // 4. We need to find where the NEXT section starts to preserve it.
    // In our simplified MVP, we'll assume the item list was the last thing.
    // BUT! Corpses come next.
    // Let's find the 'JM' marker after the items.

    // 5. Construct new save
    let mut final_bits = Vec::new();
    final_bits.extend(&all_bits[0..list_start_bit]);
    final_bits.extend(new_item_list_bits);

    // To be safe, we'll just append an empty corpse list (JM 0 0)
    for i in 0..16 {
        final_bits.push((0x4D4Au16 >> i) & 1 != 0);
    }
    for i in 0..16 {
        final_bits.push(false);
    } // Count 0

    // Convert back to bytes
    let mut writer = BitWriter::endian(Vec::new(), LittleEndian);
    for bit in final_bits {
        writer.write_bit(bit)?;
    }
    writer.byte_align()?;
    let mut result_bytes = writer.into_writer();

    // 6. Fix checksum
    Checksum::fix(&mut result_bytes);

    fs::create_dir_all("tests/fixtures/savegames/modified")?;
    fs::write(
        "tests/fixtures/savegames/modified/amazon_10_scrolls.d2s",
        result_bytes,
    )?;
    println!("Created amazon_10_scrolls.d2s");

    Ok(())
}
