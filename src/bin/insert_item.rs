use bitstream_io::{BitRead, BitReader, BitWrite, BitWriter, LittleEndian};
use d2r_core::inventory::InventoryGrid;
use d2r_core::item::{Checksum, HuffmanTree, Item};
use std::fs;
use std::io::Cursor;

fn get_item_size(code: &str) -> (u8, u8) {
    match code {
        "tsc " | "isc " | "hp1 " | "mp1 " | "vps " | "key " | "aqv " => (1, 1),
        "jav " => (1, 3),
        "buc " => (2, 2),
        "wwa7" => (1, 3),
        _ => (1, 1), // Default
    }
}

fn main() -> std::io::Result<()> {
    let save_path = "tests/fixtures/savegames/original/amazon_initial.d2s";
    let d2i_path = "tests/fixtures/items/portal_scroll.d2i";

    let mut save_bytes = fs::read(save_path).expect("Save file not found");
    let d2i_bytes = fs::read(d2i_path).expect("Item file not found");

    let huffman = HuffmanTree::new();
    let mut grid = InventoryGrid::new();

    // 1. Scan existing items and fill grid
    let item_starts = Item::scan_items(&save_bytes, &huffman);
    println!("Scanning {} existing items...", item_starts.len());

    let mut last_item_end = 0;
    for (start, code) in item_starts {
        let mut reader = BitReader::endian(Cursor::new(&save_bytes), LittleEndian);
        let _ = reader.skip((start - 53) as u32);

        let _flags: u32 = reader.read::<32, u32>()?;
        let _version: u16 = reader.read::<3, u16>()?;
        let _mode: u8 = reader.read::<3, u8>()?;
        let _loc: u8 = reader.read::<4, u8>()?;
        let x: u8 = reader.read::<4, u8>()?;
        let y: u8 = reader.read::<4, u8>()?;
        let page: u8 = reader.read::<3, u8>()?;

        if page == 1 {
            // Inventory
            let (w, h) = get_item_size(&code);
            println!(
                "  Found '{}' in Inventory at ({}, {}), Size {}x{}",
                code, x, y, w, h
            );
            grid.occupy(x, y, w, h);
        }

        // We need to estimate where the item list ends.
        // For simplicity, let's say the last item we find is near the end.
        // In a real parser, we'd follow the chain.
        last_item_end = start + 50; // Approximated
    }

    // 2. Find free slot for 1x1 portal scroll
    let (target_x, target_y) = grid.find_free_slot(1, 1).expect("No free slot available");
    println!("Inserting new item at ({}, {})", target_x, target_y);

    // 3. Prepare the new item bitstream
    // We'll take the first 70 bits of d2i_bytes (our portal scroll)
    // and patch the X, Y, Page bits.
    // D2R Item Header: Flags(32) + Version(3) + Mode(3) + Loc(4) + X(4) + Y(4) + Page(3)
    // Bits for X: 32+3+3+4 = 42 to 45
    // Bits for Y: 46 to 49
    // Bits for Page: 50 to 52

    let mut item_bits = d2i_bytes.clone();
    // Path X
    for i in 0..4 {
        Item::set_bit(&mut item_bits, 42 + i, (target_x >> i) & 1 != 0);
    }
    // Patch Y
    for i in 0..4 {
        Item::set_bit(&mut item_bits, 46 + i, (target_y >> i) & 1 != 0);
    }
    // Patch Page = 1 (Inventory)
    Item::set_bit(&mut item_bits, 50, true);
    Item::set_bit(&mut item_bits, 51, false);
    Item::set_bit(&mut item_bits, 52, false);
    // Clear 'IsNew' flag at bit 13
    Item::set_bit(&mut item_bits, 13, false);

    // 4. Construct the new save file
    // Find the 'JM' marker at 903
    let item_list_start = 903;
    let old_count = u16::from_le_bytes(
        save_bytes[item_list_start + 2..item_list_start + 4]
            .try_into()
            .unwrap(),
    );
    println!("Old item count: {}", old_count);

    // To correctly insert, we SHOULD find the exact bit end of the list.
    // For this MVP, we will use a "hack": we know where the items end in amazon_initial because they are the last thing before the CRC.
    // Actually, let's just use the end of the last item we scanned.

    // Re-scanning to find the exact bit end of the last item.
    // ... skipping complex bit-splicing for a moment and doing a byte-level append for the MVP ...

    // Correct way:
    // New Header with Count + 1
    let mut new_save = save_bytes.clone();
    let new_count = old_count + 1;
    new_save[item_list_start + 2..item_list_start + 4].copy_from_slice(&new_count.to_le_bytes());

    // Append the 70 bits of the item.
    // This is the tricky part without a full bitstream reserializer.
    // For now, let's just overwrite one of the 'wwa7' items that we know is at a fixed position.

    Ok(())
}
