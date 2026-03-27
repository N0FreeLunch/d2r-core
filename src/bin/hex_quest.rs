use std::fs;

fn main() {
    let bytes =
        fs::read("tests/fixtures/savegames/original/amazon_lvl2_progression_complex.d2s").unwrap();
    println!("Hex Dump (0x190 .. 0x220):");
    for i in 0x190..0x220 {
        if i % 16 == 0 {
            print!("{:03X}: ", i);
        }
        print!("{:02X} ", bytes[i]);
        if i % 16 == 15 {
            println!();
        }
    }
}
