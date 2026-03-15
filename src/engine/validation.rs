use crate::data::item_specs::{Affix, ItemStatRange, Runeword, SetItem, UniqueItem};
use crate::data::{affixes, runewords, set_items, unique_items};
use crate::item::{Item, ItemProperty, ItemQuality};

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub spec_name: String,
    pub is_perfect: bool,
    pub score: f32, // Overall perfection score
    pub stats: Vec<StatValidation>,
}

#[derive(Debug, Clone)]
pub struct StatValidation {
    pub stat_id: u32,
    pub name: String,
    pub param: u32,
    pub current: i32,
    pub min: i32,
    pub max: i32,
    pub is_perfect: bool,
    pub score: f32,
    pub status: StatValidationStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatValidationStatus {
    InRange,
    OutOfRange,
    MissingOnItem,
    UnexpectedOnItem,
}

pub enum ItemSpec {
    Unique(&'static UniqueItem),
    Set(&'static SetItem),
    Runeword(&'static Runeword),
    Affix(&'static Affix),
}

impl ItemSpec {
    pub fn name(&self) -> &str {
        match self {
            ItemSpec::Unique(ui) => ui.index,
            ItemSpec::Set(si) => si.index,
            ItemSpec::Runeword(rw) => rw.name,
            ItemSpec::Affix(a) => a.name,
        }
    }
}

pub fn lookup_prefix(id: u16) -> Option<&'static Affix> {
    affixes::PREFIXES.iter().find(|a| a.id == id as u32)
}

pub fn lookup_suffix(id: u16) -> Option<&'static Affix> {
    affixes::SUFFIXES.iter().find(|a| a.id == id as u32)
}

pub fn lookup_spec(item: &Item) -> Option<ItemSpec> {
    if item.is_runeword {
        if let Some(rw_id) = item.runeword_id {
            return runewords::RUNEWORDS.iter()
                .find(|rw| rw.id == rw_id as u32)
                .map(ItemSpec::Runeword);
        }
    }

    match item.quality {
        Some(ItemQuality::Unique) => {
            if let Some(uid) = item.unique_id {
                return unique_items::UNIQUE_ITEMS.iter()
                    .find(|ui| ui.id == uid as u32)
                    .map(ItemSpec::Unique);
            }
        }
        Some(ItemQuality::Set) => {
            if let Some(sid) = item.unique_id {
                return set_items::SET_ITEMS.iter()
                    .find(|si| si.id == sid as u32)
                    .map(ItemSpec::Set);
            }
        }
        _ => {}
    }
    None
}

pub fn validate_item(item: &Item) -> Option<ValidationResult> {
    if let Some(spec) = lookup_spec(item) {
        return match spec {
            ItemSpec::Unique(unique_spec) => Some(validate_item_properties(
                unique_spec.index,
                unique_spec.stats,
                &item.properties,
            )),
            ItemSpec::Set(set_spec) => Some(validate_item_properties(
                set_spec.index,
                set_spec.stats,
                &item.properties,
            )),
            ItemSpec::Runeword(rw_spec) => Some(validate_item_properties(
                rw_spec.name,
                rw_spec.stats,
                &item.runeword_attributes,
            )),
            ItemSpec::Affix(_) => None,
        };
    }

    match item.quality {
        Some(ItemQuality::Magic) | Some(ItemQuality::Rare) | Some(ItemQuality::Crafted) => {
            let prefixes = item.prefixes();
            let suffixes = item.suffixes();

            if prefixes.is_empty() && suffixes.is_empty() {
                return None;
            }

            let consolidated_stats = consolidate_affix_stats(&prefixes, &suffixes);
            let names: Vec<&str> = prefixes
                .iter()
                .chain(suffixes.iter())
                .map(|a| a.name)
                .collect();
            let spec_name = if names.is_empty() {
                "Unknown Affix Item".to_string()
            } else {
                names.join(" ")
            };

            Some(validate_item_properties(
                &spec_name,
                &consolidated_stats,
                &item.properties,
            ))
        }
        _ => None,
    }
}

fn consolidate_affix_stats(prefixes: &[&Affix], suffixes: &[&Affix]) -> Vec<ItemStatRange> {
    let mut merged: std::collections::HashMap<(u32, u32), ItemStatRange> =
        std::collections::HashMap::new();

    let all_affixes = prefixes.iter().chain(suffixes.iter());
    for affix in all_affixes {
        for spec_stat in affix.stats {
            let key = (spec_stat.stat_id, spec_stat.param);
            let entry = merged.entry(key).or_insert(ItemStatRange {
                stat_id: spec_stat.stat_id,
                param: spec_stat.param,
                min: 0,
                max: 0,
            });
            entry.min += spec_stat.min;
            entry.max += spec_stat.max;
        }
    }

    merged.into_values().collect()
}

fn validate_item_properties(
    spec_name: &str,
    spec_stats: &[ItemStatRange],
    item_properties: &[ItemProperty],
) -> ValidationResult {
    let mut stats: Vec<StatValidation> = Vec::new();
    let mut used_specs = vec![false; spec_stats.len()];

    for prop in item_properties {
        if let Some(spec_idx) = find_matching_spec_index(prop, spec_stats, &used_specs) {
            used_specs[spec_idx] = true;
            let spec_stat = &spec_stats[spec_idx];
            let (score, in_range, is_perfect) = calculate_score(prop.value, spec_stat.min, spec_stat.max);

            stats.push(StatValidation {
                stat_id: prop.stat_id,
                name: prop.name.clone(),
                param: prop.param,
                current: prop.value,
                min: spec_stat.min,
                max: spec_stat.max,
                is_perfect,
                score,
                status: if in_range {
                    StatValidationStatus::InRange
                } else {
                    StatValidationStatus::OutOfRange
                },
            });
        } else {
            stats.push(StatValidation {
                stat_id: prop.stat_id,
                name: prop.name.clone(),
                param: prop.param,
                current: prop.value,
                min: 0,
                max: 0,
                is_perfect: false,
                score: 0.0,
                status: StatValidationStatus::UnexpectedOnItem,
            });
        }
    }

    for (idx, spec_stat) in spec_stats.iter().enumerate() {
        if used_specs[idx] {
            continue;
        }

        stats.push(StatValidation {
            stat_id: spec_stat.stat_id,
            name: format!("stat_{}", spec_stat.stat_id),
            param: spec_stat.param,
            current: 0,
            min: spec_stat.min,
            max: spec_stat.max,
            is_perfect: false,
            score: 0.0,
            status: StatValidationStatus::MissingOnItem,
        });
    }

    let variable_scores: Vec<f32> = stats
        .iter()
        .filter(|entry| {
            (entry.status == StatValidationStatus::InRange
                || entry.status == StatValidationStatus::OutOfRange)
                && entry.min != entry.max
        })
        .map(|entry| entry.score)
        .collect();

    let score = if !variable_scores.is_empty() {
        variable_scores.iter().sum::<f32>() / variable_scores.len() as f32
    } else {
        let matched_scores: Vec<f32> = stats
            .iter()
            .filter(|entry| {
                entry.status == StatValidationStatus::InRange
                    || entry.status == StatValidationStatus::OutOfRange
            })
            .map(|entry| entry.score)
            .collect();
        if matched_scores.is_empty() {
            0.0
        } else {
            matched_scores.iter().sum::<f32>() / matched_scores.len() as f32
        }
    };

    let has_only_in_range = stats
        .iter()
        .all(|entry| entry.status == StatValidationStatus::InRange);
    let variable_perfect = stats
        .iter()
        .filter(|entry| entry.min != entry.max)
        .all(|entry| entry.is_perfect);
    let is_perfect = !stats.is_empty() && has_only_in_range && variable_perfect;

    ValidationResult {
        spec_name: spec_name.to_string(),
        is_perfect,
        score,
        stats,
    }
}

fn find_matching_spec_index(
    prop: &ItemProperty,
    spec_stats: &[ItemStatRange],
    used_specs: &[bool],
) -> Option<usize> {
    spec_stats
        .iter()
        .enumerate()
        .find_map(|(idx, spec_stat)| {
            if used_specs[idx] {
                return None;
            }

            if spec_stat.stat_id == prop.stat_id && spec_stat.param == prop.param {
                Some(idx)
            } else {
                None
            }
        })
}

fn calculate_score(current: i32, min: i32, max: i32) -> (f32, bool, bool) {
    if min == max {
        let in_range = current == min;
        let score = if in_range { 1.0 } else { 0.0 };
        return (score, in_range, in_range);
    }

    if min < max {
        if current < min || current > max {
            return (0.0, false, false);
        }

        let range = (max - min) as f32;
        let ratio = ((current - min) as f32 / range).clamp(0.0, 1.0);
        return (ratio, true, current == max);
    }

    if current > min || current < max {
        return (0.0, false, false);
    }

    let range = (min - max) as f32;
    let ratio = ((min - current) as f32 / range).clamp(0.0, 1.0);
    (ratio, true, current == max)
}
