use d2r_core::engine::validation::{validate_item, StatValidationStatus};
use d2r_core::item::{Item, ItemProperty, ItemQuality};

fn base_item() -> Item {
    let mut item = Item::empty_for_tests();
    item.code = "hax ".to_string();
    item.is_compact = true;
    item.is_identified = true;
    item.properties_complete = true;
    item
}

fn prop(stat_id: u32, param: u32, value: i32) -> ItemProperty {
    ItemProperty {
        stat_id,
        name: format!("stat_{stat_id}"),
        param,
        raw_value: value,
        value,
    }
}

#[test]
fn validate_unique_item_perfect_roll() {
    let mut item = base_item();
    item.quality = Some(ItemQuality::Unique);
    item.unique_id = Some(0); // The Gnasher
    item.properties = vec![
        prop(0, 0, 8),
        prop(135, 0, 50),
        prop(136, 0, 20),
        prop(111, 0, 70),
    ];

    let result = validate_item(&item).expect("unique item should resolve a spec");
    assert_eq!(result.spec_name, "The Gnasher");
    assert!(result.is_perfect);
    assert!((result.score - 1.0).abs() < f32::EPSILON);
    assert!(result.stats.iter().all(|s| s.status == StatValidationStatus::InRange));
}

#[test]
fn validate_unique_item_out_of_range_roll() {
    let mut item = base_item();
    item.quality = Some(ItemQuality::Unique);
    item.unique_id = Some(0); // The Gnasher
    item.properties = vec![
        prop(0, 0, 8),
        prop(135, 0, 50),
        prop(136, 0, 20),
        prop(111, 0, 80), // out of range (60..70)
    ];

    let result = validate_item(&item).expect("unique item should resolve a spec");
    assert_eq!(result.spec_name, "The Gnasher");
    assert!(!result.is_perfect);
    assert!(result.score < 1.0);
    assert!(result
        .stats
        .iter()
        .any(|s| s.stat_id == 111 && s.status == StatValidationStatus::OutOfRange));
}

#[test]
fn validate_runeword_item_static_data() {
    let mut item = base_item();
    item.code = "tors".to_string();
    item.is_runeword = true;
    item.runeword_id = Some(2); // Authority
    item.runeword_attributes = vec![
        prop(201, 387, 10),
        prop(198, 399, 12),
        prop(83, 0, 2),
        prop(111, 0, 50),
    ];

    let result = validate_item(&item).expect("runeword item should resolve a spec");
    assert_eq!(result.spec_name, "Authority");
    assert!(!result.is_perfect);
    assert!(result.score > 0.0 && result.score < 1.0);
    assert!(result
        .stats
        .iter()
        .all(|s| s.status == StatValidationStatus::InRange));
}

#[test]
fn validate_param_mismatch_creates_missing_and_unexpected() {
    let mut item = base_item();
    item.code = "tors".to_string();
    item.is_runeword = true;
    item.runeword_id = Some(2); // Authority
    item.runeword_attributes = vec![
        prop(201, 0, 10), // wrong param (expected 387)
        prop(198, 399, 12),
        prop(83, 0, 2),
        prop(111, 0, 50),
    ];

    let result = validate_item(&item).expect("runeword item should resolve a spec");
    assert_eq!(result.spec_name, "Authority");
    assert!(result
        .stats
        .iter()
        .any(|s| s.status == StatValidationStatus::UnexpectedOnItem));
    assert!(result
        .stats
        .iter()
        .any(|s| s.status == StatValidationStatus::MissingOnItem));
}
