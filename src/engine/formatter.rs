use crate::data::localization::LOCALIZATIONS;
use crate::data::monsters::MONSTER_TYPES;
use crate::data::skills::SKILLS;
use crate::data::stat_costs::STAT_COSTS;
use crate::item::{Item, ItemProperty};

const CLASS_NAMES_EN: [&str; 8] = [
    "Amazon",
    "Sorceress",
    "Necromancer",
    "Paladin",
    "Barbarian",
    "Druid",
    "Assassin",
    "Warlock",
];

const CLASS_NAMES_KO: [&str; 8] = [
    "아마존",
    "소서리스",
    "네크로맨서",
    "팔라딘",
    "바바리안",
    "드루이드",
    "어쌔신",
    "워락",
];

const SKILL_TABS_EN: [[&str; 3]; 8] = [
    ["Javelin and Spear", "Passive and Magic", "Bow and Crossbow"],
    ["Fire", "Lightning", "Cold"],
    ["Curses", "Poison and Bone", "Summoning"],
    ["Combat Skills", "Offensive Auras", "Defensive Auras"],
    ["Combat Skills", "Combat Masteries", "Warcries"],
    ["Summoning", "Shape Shifting", "Elemental"],
    ["Martial Arts", "Shadow Disciplines", "Traps"],
    ["Warlock Tab 1", "Warlock Tab 2", "Warlock Tab 3"],
];

const SKILL_TABS_KO: [[&str; 3]; 8] = [
    ["재벌린 & 스피어", "패시브 & 매직", "활 & 쇠뇌"],
    ["화염", "번개", "냉기"],
    ["저주", "포이즌 & 본", "소환"],
    ["전투 기술", "공격 오라", "방어 오라"],
    ["전투 기술", "전투 숙련", "함성"],
    ["소환", "변신", "원소"],
    ["무술", "그림자 단련", "덫"],
    ["워락 탭 1", "워락 탭 2", "워락 탭 3"],
];

pub struct FormattedItem {
    pub name: String,
    pub level: u8,
    pub quality_name: String,
    pub base_attributes: Vec<String>,
    pub properties: Vec<String>,
}

pub fn format_item(item: &Item, language: &str) -> FormattedItem {
    let mut base_attributes = Vec::new();

    if let Some(def) = item.defense {
        let label = match language {
            "ko" => "방어력",
            _ => "Defense",
        };
        base_attributes.push(format!("{}: {}", label, def));
    }

    if let (Some(cur), Some(max)) = (item.current_durability, item.max_durability) {
        if max > 0 {
            let label = match language {
                "ko" => "내구도",
                _ => "Durability",
            };
            base_attributes.push(format!("{}: {} / {}", label, cur, max));
        }
    }

    if let Some(qty) = item.quantity {
        let label = match language {
            "ko" => "수량",
            _ => "Quantity",
        };
        base_attributes.push(format!("{}: {}", label, qty));
    }

    if item.is_ethereal {
        let label = match language {
            "ko" => "회복 불가 (에테리얼)",
            _ => "Ethereal (Cannot be Repaired)",
        };
        base_attributes.push(label.to_string());
    }

    if let Some(s) = item.sockets {
        if s > 0 {
            let label = match language {
                "ko" => format!("소켓 ({})", s),
                _ => format!("Socketed ({})", s),
            };
            base_attributes.push(label);
        }
    }

    let properties = item
        .properties
        .iter()
        .map(|p| format_property(p, language))
        .collect();

    FormattedItem {
        name: item.code.clone(),
        level: item.level.unwrap_or(0),
        quality_name: format!("{:?}", item.quality),
        base_attributes,
        properties,
    }
}

pub fn format_property(prop: &ItemProperty, language: &str) -> String {
    let stat_id = prop.stat_id as u32;
    let Some(cost) = STAT_COSTS.get(stat_id as usize) else {
        return format!("Unknown Stat {} (value: {})", stat_id, prop.value);
    };

    let descfunc = cost.descfunc.unwrap_or(0);
    let key = if prop.value >= 0 {
        cost.descstrpos
    } else {
        cost.descstrneg
    };
    let Some(key) = key else {
        return format!("{} (value: {})", cost.name, prop.value);
    };

    let loc_str = get_loc(key, language).unwrap_or(key).trim();
    let signed_value = format!("{:+}", prop.value);

    match descfunc {
        1 | 19 => format!("{} {}", signed_value, loc_str),
        2 => format!("{}% {}", prop.value, loc_str),
        3 => format!("{} {}", prop.value, loc_str),
        4 | 8 => format!("{}% {}", signed_value, loc_str),
        5 => format!("{} {}%", loc_str, prop.value),
        6 => format!("{}% {}", signed_value, loc_str),
        7 => format!("{}% {}", prop.value, loc_str),
        11 => {
            if loc_str.contains('%') {
                format_template(loc_str, &[prop.value.to_string()])
            } else {
                format!("{} {}", prop.value, loc_str)
            }
        }
        12 => {
            if loc_str.contains('%') {
                format_template(loc_str, &[prop.value.to_string()])
            } else {
                format!("{} {}", loc_str, signed_value)
            }
        }
        13 => {
            let class_id = prop.param as usize;
            if class_id == 0 && !loc_str.starts_with("ItemModifier") {
                format!("{} {}", signed_value, loc_str)
            } else if let Some(class_name) = class_name(class_id, language) {
                if language == "ko" {
                    format!("{} {} 스킬 레벨 상승", signed_value, class_name)
                } else {
                    format!("{} to {} Skill Levels", signed_value, class_name)
                }
            } else {
                format!("{} {} (class #{})", signed_value, loc_str, class_id)
            }
        }
        14 => {
            let (class_id, tab_index) = decode_skill_tab_param(prop.param);
            if class_id == 0 && tab_index == 0 && loc_str.contains('%') {
                format_template(loc_str, &[prop.value.to_string()])
            } else if let Some(tab_name) = skill_tab_name(class_id, tab_index, language) {
                if language == "ko" {
                    format!("{} {} 스킬", signed_value, tab_name)
                } else {
                    format!("{} to {} Skills", signed_value, tab_name)
                }
            } else {
                format!(
                    "{} {} (class #{}, tab #{})",
                    signed_value, loc_str, class_id, tab_index
                )
            }
        }
        15 => {
            let (skill_id, skill_level) = decode_skill_param(prop.param);
            let skill = skill_name(skill_id, language);
            if loc_str.contains('%') {
                format_template(
                    loc_str,
                    &[
                        prop.value.to_string(),
                        skill_level.to_string(),
                        skill.to_string(),
                    ],
                )
            } else {
                // descfunc 15 appends a descstr2 phrase (e.g. "on attack")
                let phrase2 = STAT_COSTS
                    .iter()
                    .find(|s| s.id == prop.stat_id)
                    .and_then(|s| s.descstr2)
                    .and_then(|k| get_loc(k, language))
                    .unwrap_or(if language == "ko" { "공격 시" } else { "on attack" });
                if language == "ko" {
                    format!("{}% 확률로 레벨 {} {} {} 시전", prop.value, skill_level, skill, phrase2)
                } else {
                    format!("{}% Chance to cast level {} {} {}", prop.value, skill_level, skill, phrase2)
                }
            }
        }
        16 => {
            let skill = skill_name(prop.param, language);
            if loc_str.contains('%') {
                format_template(loc_str, &[prop.value.to_string(), skill.to_string()])
            } else if language == "ko" {
                format!("장착 시 레벨 {} {} 오라", prop.value, skill)
            } else {
                format!("Level {} {} Aura When Equipped", prop.value, skill)
            }
        }
        17 => {
            if language == "ko" {
                format!("{} {} (시간 경과에 따라 증가)", prop.value, loc_str)
            } else {
                format!("{} {} (Increases over time)", prop.value, loc_str)
            }
        }
        18 => {
            if language == "ko" {
                format!("{}% {} (시간 경과에 따라 증가)", prop.value, loc_str)
            } else {
                format!("{}% {} (Increases over time)", prop.value, loc_str)
            }
        }
        20 => format!("-{}% {}", prop.value.abs(), loc_str),
        21 => format!("-{} {}", prop.value.abs(), loc_str),
        22 => {
            let monster = monster_type_name(prop.param, language);
            if cost.name == "attack_vs_montype" {
                format!("{} {} {}", signed_value, loc_str, monster)
            } else {
                format!("{}% {} {}", signed_value, loc_str, monster)
            }
        }
        23 => {
            let monster = if language == "ko" {
                format!("몬스터 #{}", prop.param)
            } else {
                format!("Monster #{}", prop.param)
            };
            format!("{}% {} {}", prop.value, loc_str, monster)
        }
        24 => {
            let (skill_id, skill_level) = decode_skill_param(prop.param);
            let skill = skill_name(skill_id, language);
            let current_charges = prop.value & 0xFF;
            let max_charges = (prop.value >> 8) & 0xFF;
            let charge_text = if loc_str.contains('%') {
                format_template(
                    loc_str,
                    &[current_charges.to_string(), max_charges.to_string()],
                )
            } else {
                format!("({}/{})", current_charges, max_charges)
            };

            if language == "ko" {
                format!("레벨 {} {} {}", skill_level, skill, charge_text)
            } else {
                format!("Level {} {} {}", skill_level, skill, charge_text)
            }
        }
        27 => {
            let skill = skill_name(prop.param, language);
            if language == "ko" {
                format!("{} {} (클래스 전용)", signed_value, skill)
            } else {
                format!("{} to {} (Class Only)", signed_value, skill)
            }
        }
        28 => {
            let skill = skill_name(prop.param, language);
            if language == "ko" {
                format!("{} {}", signed_value, skill)
            } else {
                format!("{} to {}", signed_value, skill)
            }
        }
        29 => {
            if cost.name == "damageresist" {
                if prop.value >= 0 {
                    if language == "ko" {
                        format!("피해 감소 {}%", prop.value)
                    } else {
                        format!("Damage Reduced by {}%", prop.value)
                    }
                } else if language == "ko" {
                    format!("받는 피해 증가 {}%", prop.value.abs())
                } else {
                    format!("Damage Increased by {}%", prop.value.abs())
                }
            } else if loc_str.contains('%') {
                format_template(loc_str, &[prop.value.to_string()])
            } else {
                format!("{} {}", signed_value, loc_str)
            }
        }
        _ => format!("{} (func {}): {}", loc_str, descfunc, prop.value),
    }
}

fn class_name(class_id: usize, language: &str) -> Option<&'static str> {
    if language == "ko" {
        CLASS_NAMES_KO.get(class_id).copied()
    } else {
        CLASS_NAMES_EN.get(class_id).copied()
    }
}

fn skill_tab_name(class_id: usize, tab_index: usize, language: &str) -> Option<&'static str> {
    if language == "ko" {
        SKILL_TABS_KO
            .get(class_id)
            .and_then(|tabs| tabs.get(tab_index))
            .copied()
    } else {
        SKILL_TABS_EN
            .get(class_id)
            .and_then(|tabs| tabs.get(tab_index))
            .copied()
    }
}

fn skill_name(skill_id: u32, language: &str) -> String {
    let key = SKILLS.iter().find(|s| s.id == skill_id).map(|s| s.key);
    if let Some(k) = key {
        if let Some(loc) = get_loc(k, language) {
            return loc.to_string();
        }
    }

    if language == "ko" {
        format!("기술 #{}", skill_id)
    } else {
        format!("Skill #{}", skill_id)
    }
}

fn monster_type_name(montype_id: u32, language: &str) -> String {
    let key = MONSTER_TYPES.iter().find(|m| m.id == montype_id).map(|m| m.key);
    if let Some(k) = key {
        if let Some(loc) = get_loc(k, language) {
            return loc.to_string();
        }
    }

    if language == "ko" {
        format!("몬스터 유형 #{}", montype_id)
    } else {
        format!("Monster Type #{}", montype_id)
    }
}

fn decode_skill_param(param: u32) -> (u32, u32) {
    let skill_level = param & 0x3F;
    let skill_id = param >> 6;
    (skill_id, skill_level)
}

fn decode_skill_tab_param(param: u32) -> (usize, usize) {
    let packed_class = (param >> 3) as usize;
    let packed_tab = (param & 0x7) as usize;
    if packed_class < CLASS_NAMES_EN.len() && packed_tab < 3 {
        return (packed_class, packed_tab);
    }

    let absolute = (param & 0xFF) as usize;
    (absolute / 3, absolute % 3)
}

fn format_template(template: &str, args: &[String]) -> String {
    let mut out = String::new();
    let mut chars = template.chars().peekable();
    let mut arg_idx = 0usize;

    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }

        if matches!(chars.peek(), Some('%')) {
            chars.next();
            out.push('%');
            continue;
        }

        let force_plus = if matches!(chars.peek(), Some('+')) {
            chars.next();
            true
        } else {
            false
        };

        let Some(spec) = chars.next() else {
            out.push('%');
            if force_plus {
                out.push('+');
            }
            break;
        };

        if spec != 'd' && spec != 's' {
            out.push('%');
            if force_plus {
                out.push('+');
            }
            out.push(spec);
            continue;
        }

        let value = args.get(arg_idx).cloned().unwrap_or_default();
        arg_idx += 1;
        if force_plus && !value.starts_with('-') && !value.starts_with('+') {
            out.push('+');
        }
        out.push_str(&value);
    }

    out
}

fn get_loc(key: &str, language: &str) -> Option<&'static str> {
    LOCALIZATIONS
        .iter()
        .find(|l| l.key == key)
        .map(|l| if language == "ko" { l.ko } else { l.en })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::HuffmanTree;
    use std::fs;
    use std::path::PathBuf;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    fn make_prop(stat_id: u32, param: u32, value: i32) -> ItemProperty {
        ItemProperty {
            stat_id,
            name: String::new(),
            param,
            raw_value: value,
            value,
        }
    }

    #[test]
    fn test_format_unique_axe() {
        let bytes = fs::read(repo_path(
            "tests/fixtures/savegames/original/amazon_10_scrolls.d2s",
        ))
        .expect("fixture should exist");
        let huffman = HuffmanTree::new();
        let items = Item::read_player_items(&bytes, &huffman).expect("items should parse");

        let buckler = &items[15];
        let formatted_en = format_item(buckler, "en");
        let formatted_ko = format_item(buckler, "ko");

        assert!(!formatted_en.base_attributes.is_empty());
        assert!(formatted_en.base_attributes[0].contains("Defense"));
        assert!(formatted_ko.base_attributes[0].contains("방어력"));
    }

    #[test]
    fn test_descfunc_13_class_skills() {
        let prop = make_prop(83, 1, 2);
        assert_eq!(format_property(&prop, "en"), "+2 to Sorceress Skill Levels");
    }

    #[test]
    fn test_descfunc_14_skill_tab() {
        let prop = make_prop(188, 0, 3);
        assert_eq!(
            format_property(&prop, "en"),
            "+3 to Javelin and Spear Skills"
        );
    }

    #[test]
    fn test_descfunc_15_chance_to_cast() {
        let param = (5 << 6) | 3;
        let prop = make_prop(195, param, 25);
        assert_eq!(
            format_property(&prop, "en"),
            "25% Chance to cast level 3 Skill #5 on attack"
        );
    }

    #[test]
    fn test_descfunc_24_charged_skill() {
        let param = (7 << 6) | 2;
        let value = (20 << 8) | 5;
        let prop = make_prop(204, param, value);
        assert_eq!(
            format_property(&prop, "en"),
            "Level 2 Skill #7 (5/20 Charges)"
        );
    }

    #[test]
    fn test_descfunc_22_vs_monster_type() {
        let prop = make_prop(179, 4, 120);
        assert_eq!(
            format_property(&prop, "en"),
            "+120 to Attack Rating versus Monster Type #4"
        );
    }

    #[test]
    fn test_descfunc_29_damage_resist() {
        let prop = make_prop(36, 0, 15);
        assert_eq!(format_property(&prop, "en"), "Damage Reduced by 15%");
    }
}
