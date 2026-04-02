use bitstream_io::{BitReader, LittleEndian};
use d2r_core::item::{BitRecorder, HuffmanTree};
use std::fs;
use std::io::Cursor;

fn main() {
    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_authority_runeword.d2s").unwrap();
    let item_start_bit = 7560;
    let huffman = HuffmanTree::new();

    // Offset 230 is at 7790
    let bit_pos = 7790;
    let byte_offset = bit_pos / 8;
    let bit_offset = bit_pos % 8;

    let mut reader = BitReader::endian(Cursor::new(&bytes[byte_offset as usize..]), LittleEndian);
    let mut recorder = BitRecorder::new(&mut reader);
    for _ in 0..bit_offset {
        recorder.read_bit().ok();
    }

    let id_bits = 9;
    let stat_id = recorder.read_bits(id_bits).unwrap();
    println!("Stat ID read at 7790 with 9 bits: {}", stat_id);

    let result =
        d2r_core::item::read_property_list(&mut recorder, "xrs ", 5, None, &huffman, false);
    match result {
        Ok((props, _, _)) => {
            println!("Props: {}", props.len());
            for p in props {
                println!("  ID {}: Val {}", p.stat_id, p.value);
            }
        }
        Err(e) => println!("Error: {:?}", e),
    }
}
