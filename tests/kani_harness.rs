#[cfg(kani)]
mod kani_tests {
    use crate::domain::vo::*;

    #[kani::proof]
    fn proof_item_stat_value_invariants() {
        let val: i32 = kani::any();
        let result = ItemStatValue::new(val);
        
        if val >= ItemStatValue::MIN_SAFE && val <= ItemStatValue::MAX_SAFE {
            assert!(result.is_ok());
            assert_eq!(result.unwrap().value(), val);
        } else {
            assert!(result.is_err());
        }
    }

    #[kani::proof]
    fn proof_inventory_coordinate_invariants() {
        let x: u8 = kani::any();
        let y: u8 = kani::any();
        let result = InventoryCoordinate::new(x, y);

        if x <= InventoryCoordinate::MAX_X && y <= InventoryCoordinate::MAX_Y {
            assert!(result.is_ok());
            let vo = result.unwrap();
            assert_eq!(vo.x(), x);
            assert_eq!(vo.y(), y);
        } else {
            assert!(result.is_err());
        }
    }

    #[kani::proof]
    fn proof_item_quality_mapping_is_total() {
        let val: u8 = kani::any();
        // Since ItemQuality::from(v) uses a match with a wildcard _,
        // it is a total function and should never panic.
        let _quality = d2r_core::item::map_item_quality(val);
    }

    #[kani::proof]
    fn proof_calculate_stat_value_is_safe() {
        let raw: i32 = kani::any();
        let save_add: i32 = kani::any();
        // wrapping_sub is defined for all i32 combinations.
        let _ = d2r_core::item::calculate_stat_value(raw, save_add);
    }
}
