//! Every value a person can pick for a setting, with the text to show. The picker on the desktop and
//! the menu in Deck mode both list exactly this, so the two cannot disagree about what is on offer.
use super::value::{SettingValue, ValueKind};

/// The longest list a picker shows for a numeric range. A longer range is thinned, keeping both ends.
const MAX_STEPS: usize = 12;

/// Everything selectable for `kind`, in display order, each with its label.
pub fn options(kind: &ValueKind) -> Vec<(SettingValue, String)> {
    match kind {
        ValueKind::Bool => vec![(SettingValue::Bool(true), "On".into()), (SettingValue::Bool(false), "Off".into())],
        ValueKind::Choice(choices) => choices.iter().map(|c| (SettingValue::Choice(c.id.clone()), c.label.clone())).collect(),
        ValueKind::Size { options, native } => {
            let mut all: Vec<(SettingValue, String)> = Vec::new();
            if *native {
                all.push((SettingValue::Native, "Native".into()));
            }
            all.extend(options.iter().map(|s| (SettingValue::Size(*s), s.label())));
            all
        }
        ValueKind::Int { min, max, step, unit, zero_label } => {
            let step = (*step).max(1);
            let count = ((max - min) / step + 1).max(1) as usize;
            let stride = count.div_ceil(MAX_STEPS).max(1);
            let mut values: Vec<i64> = (0..count).step_by(stride).map(|i| min + i as i64 * step).collect();
            // The top of the range is always offered, even when the stride skips over it.
            let top = min + (count as i64 - 1) * step;
            if values.last() != Some(&top) {
                values.push(top);
            }
            values.into_iter().map(|v| (SettingValue::Int(v), int_label(v, *unit, *zero_label))).collect()
        }
    }
}

fn int_label(value: i64, unit: Option<&str>, zero_label: Option<&str>) -> String {
    match (value, zero_label, unit) {
        (0, Some(zero), _) => zero.to_string(),
        (v, _, Some("%")) => format!("{v}%"),
        (v, _, Some(unit)) => format!("{v} {unit}"),
        (v, _, None) => v.to_string(),
    }
}

/// The text for one value of `kind`: the label from [`options`] if it is one of them, otherwise a
/// plain rendering (a stored number between two listed steps still reads as a number).
pub fn label(kind: &ValueKind, value: &SettingValue) -> String {
    if let Some((_, label)) = options(kind).into_iter().find(|(v, _)| v == value) {
        return label;
    }
    match (kind, value) {
        (ValueKind::Int { unit, zero_label, .. }, SettingValue::Int(v)) => int_label(*v, *unit, *zero_label),
        (_, SettingValue::Size(size)) => size.label(),
        (_, SettingValue::Int(v)) => v.to_string(),
        (_, SettingValue::Choice(id)) => id.clone(),
        (_, SettingValue::Bool(b)) => if *b { "On" } else { "Off" }.to_string(),
        (_, SettingValue::Native) => "Native".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{Choice, Size};

    fn frame_cap() -> ValueKind {
        ValueKind::Int { min: 0, max: 360, step: 1, unit: Some("fps"), zero_label: Some("No limit") }
    }

    #[test]
    fn a_bool_is_on_or_off() {
        let labels: Vec<_> = options(&ValueKind::Bool).into_iter().map(|(_, l)| l).collect();
        assert_eq!(labels, vec!["On", "Off"]);
    }

    #[test]
    fn choices_keep_their_order_and_ids() {
        let kind = ValueKind::Choice(vec![Choice::new("a", "Alpha"), Choice::new("b", "Beta")]);
        assert_eq!(options(&kind), vec![(SettingValue::choice("a"), "Alpha".to_string()), (SettingValue::choice("b"), "Beta".to_string())]);
    }

    #[test]
    fn sizes_list_native_first_when_allowed() {
        let kind = ValueKind::Size { options: vec![Size::new(1280, 720), Size::new(1920, 1080)], native: true };
        let labels: Vec<_> = options(&kind).into_iter().map(|(_, l)| l).collect();
        assert_eq!(labels, vec!["Native", "1280x720", "1920x1080"]);
        let no_native = ValueKind::Size { options: vec![Size::new(1280, 720)], native: false };
        assert_eq!(options(&no_native).len(), 1);
    }

    #[test]
    fn a_long_range_is_thinned_but_keeps_both_ends_and_the_zero_label() {
        let all = options(&frame_cap());
        assert!(all.len() <= MAX_STEPS + 1, "{}", all.len());
        assert_eq!(all.first(), Some(&(SettingValue::Int(0), "No limit".to_string())));
        assert_eq!(all.last(), Some(&(SettingValue::Int(360), "360 fps".to_string())));
        assert!(all.windows(2).all(|w| matches!((&w[0].0, &w[1].0), (SettingValue::Int(a), SettingValue::Int(b)) if a < b)));
    }

    #[test]
    fn a_short_range_lists_every_step() {
        let kind = ValueKind::Int { min: 25, max: 100, step: 25, unit: Some("%"), zero_label: None };
        let labels: Vec<_> = options(&kind).into_iter().map(|(_, l)| l).collect();
        assert_eq!(labels, vec!["25%", "50%", "75%", "100%"]);
    }

    #[test]
    fn a_stored_value_between_listed_steps_still_has_a_label() {
        assert_eq!(label(&frame_cap(), &SettingValue::Int(61)), "61 fps");
        assert_eq!(label(&frame_cap(), &SettingValue::Int(0)), "No limit");
        assert_eq!(label(&ValueKind::Bool, &SettingValue::Bool(false)), "Off");
        assert_eq!(label(&ValueKind::Bool, &SettingValue::choice("odd")), "odd");
    }

    #[test]
    fn a_degenerate_range_is_one_value() {
        let kind = ValueKind::Int { min: 5, max: 5, step: 0, unit: None, zero_label: None };
        assert_eq!(options(&kind), vec![(SettingValue::Int(5), "5".to_string())]);
    }
}
