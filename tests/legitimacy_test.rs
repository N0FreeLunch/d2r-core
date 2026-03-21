use d2r_core::data::generated::legitimacy::calc_alvl;

#[test]
fn calc_alvl_matches_expected_formula() {
    // Based on TempLvl = max(ilvl, qlvl), with magic_lvl == 0.
    assert_eq!(calc_alvl(50, 30, 0), 35);
}
