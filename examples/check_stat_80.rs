use d2r_core::data::stat_costs::STAT_COSTS;

fn main() {
    let stat_id = 80;
    if let Some(c) = STAT_COSTS.iter().find(|c| c.id == stat_id as u32) {
        println!(
            "Stat {}: save_bits = {}, save_add = {}",
            stat_id, c.save_bits, c.save_add
        );
    } else {
        println!("Stat {} not found in stat_costs", stat_id);
    }
}
