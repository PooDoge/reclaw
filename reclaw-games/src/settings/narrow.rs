//! Narrowing a key's standard range by what one game accepts.
use super::{
    capabilities::Constraint,
    environment::DisplayEnvironment,
    value::{Size, ValueKind},
};

/// `kind` limited to what `constraint` allows, or `None` when nothing is left. A constraint of the
/// wrong type for the kind is ignored: it cannot narrow a range it does not describe.
pub(super) fn narrow(kind: ValueKind, constraint: &Constraint, env: &DisplayEnvironment) -> Option<ValueKind> {
    match (kind, constraint) {
        (ValueKind::Choice(options), Constraint::Choices(ids)) => {
            let kept: Vec<_> = options.into_iter().filter(|o| ids.contains(&o.id)).collect();
            (!kept.is_empty()).then_some(ValueKind::Choice(kept))
        }
        (ValueKind::Int { min, max, step, unit, zero_label }, Constraint::Range { min: lo, max: hi }) => {
            narrow_int(min, max, step, unit, zero_label, *lo, *hi)
        }
        (ValueKind::Size { options, native }, Constraint::Sizes(sizes)) => narrow_sizes(options, native, sizes, env),
        (kind, _) => Some(kind),
    }
}

/// Keeps the standard grid (`min + n*step`) so that every offered value is one `adapt` leaves alone.
fn narrow_int(
    min: i64,
    max: i64,
    step: i64,
    unit: Option<&'static str>,
    zero_label: Option<&'static str>,
    lo: i64,
    hi: i64,
) -> Option<ValueKind> {
    let step = i128::from(step.max(1));
    let (min, max) = (i128::from(min), i128::from(max));
    let (lo, hi) = (i128::from(lo).max(min), i128::from(hi).min(max));
    if lo > hi {
        return None;
    }
    let first = min + (lo - min + step - 1) / step * step;
    let last = min + (hi - min) / step * step;
    if first > last {
        return None;
    }
    // The label names 0, so it only applies while 0 can still be chosen.
    let zero_label = zero_label.filter(|_| (first..=last).contains(&0));
    Some(ValueKind::Int {
        min: i64::try_from(first).ok()?,
        max: i64::try_from(last).ok()?,
        step: i64::try_from(step).ok()?,
        unit,
        zero_label,
    })
}

/// The game's sizes that the launcher lists (and the monitor can show). "Native" survives only when
/// the primary monitor's own size is one the game accepts; a Monitor choice is not known here.
fn narrow_sizes(options: Vec<Size>, native: bool, accepted: &[Size], env: &DisplayEnvironment) -> Option<ValueKind> {
    let options: Vec<Size> = options.into_iter().filter(|s| accepted.contains(s)).collect();
    let native = native && env.primary().is_some_and(|m| accepted.contains(&m.native));
    (!options.is_empty() || native).then_some(ValueKind::Size { options, native })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::environment::DisplayServer;
    use crate::settings::{
        testing::{desktop, env, monitor},
        value::Choice,
    };

    fn choice_kind(ids: &[&str]) -> ValueKind {
        ValueKind::Choice(ids.iter().map(|id| Choice::new(id, id)).collect())
    }

    fn int(min: i64, max: i64, step: i64, zero_label: Option<&'static str>) -> ValueKind {
        ValueKind::Int { min, max, step, unit: None, zero_label }
    }

    fn range(min: i64, max: i64) -> Constraint {
        Constraint::Range { min, max }
    }

    fn no_env() -> DisplayEnvironment {
        DisplayEnvironment::unknown()
    }

    #[test]
    fn none_changes_nothing() {
        let kind = choice_kind(&["a", "b"]);
        assert_eq!(narrow(kind.clone(), &Constraint::None, &no_env()), Some(kind));
    }

    #[test]
    fn choices_keep_the_standard_order_and_drop_unknown_ids() {
        let narrowed = narrow(
            choice_kind(&["off", "2x", "4x", "8x"]),
            &Constraint::Choices(vec!["4x".into(), "nope".into(), "off".into()]),
            &no_env(),
        );
        assert_eq!(narrowed, Some(choice_kind(&["off", "4x"])));
    }

    #[test]
    fn choices_with_nothing_in_common_drop_the_setting() {
        assert_eq!(narrow(choice_kind(&["off"]), &Constraint::Choices(vec!["8x".into()]), &no_env()), None);
        assert_eq!(narrow(choice_kind(&["off"]), &Constraint::Choices(vec![]), &no_env()), None);
    }

    #[test]
    fn a_constraint_of_the_wrong_type_is_ignored() {
        let choices = choice_kind(&["a", "b"]);
        assert_eq!(narrow(choices.clone(), &range(1, 2), &no_env()), Some(choices.clone()));
        assert_eq!(narrow(choices.clone(), &Constraint::Sizes(vec![Size::new(1, 1)]), &no_env()), Some(choices));
        assert_eq!(narrow(ValueKind::Bool, &Constraint::Choices(vec!["x".into()]), &no_env()), Some(ValueKind::Bool));
        assert_eq!(narrow(int(0, 10, 1, None), &Constraint::Choices(vec!["x".into()]), &no_env()), Some(int(0, 10, 1, None)));
        let sizes = ValueKind::Size { options: vec![Size::new(1280, 720)], native: true };
        assert_eq!(narrow(sizes.clone(), &range(0, 1), &no_env()), Some(sizes));
    }

    #[test]
    fn range_intersects_with_the_standard_range() {
        assert_eq!(narrow(int(0, 360, 1, Some("No limit")), &range(0, 240), &no_env()), Some(int(0, 240, 1, Some("No limit"))));
        assert_eq!(narrow(int(0, 360, 1, None), &range(30, 1000), &no_env()), Some(int(30, 360, 1, None)));
        assert_eq!(narrow(int(0, 360, 1, None), &range(-50, 10), &no_env()), Some(int(0, 10, 1, None)));
    }

    #[test]
    fn range_stays_on_the_standard_grid() {
        // 25, 30, 35 ... : 26 rounds up to 30 and 42 rounds down to 40.
        assert_eq!(narrow(int(25, 200, 5, None), &range(26, 42), &no_env()), Some(int(30, 40, 5, None)));
        assert_eq!(narrow(int(25, 200, 5, None), &range(30, 40), &no_env()), Some(int(30, 40, 5, None)));
    }

    #[test]
    fn range_without_a_grid_point_or_overlap_drops_the_setting() {
        assert_eq!(narrow(int(25, 200, 5, None), &range(26, 29), &no_env()), None);
        assert_eq!(narrow(int(0, 100, 5, None), &range(200, 300), &no_env()), None);
        assert_eq!(narrow(int(0, 100, 5, None), &range(50, 10), &no_env()), None);
    }

    #[test]
    fn the_zero_label_goes_away_when_zero_is_not_offered() {
        assert_eq!(narrow(int(0, 360, 1, Some("No limit")), &range(30, 240), &no_env()), Some(int(30, 240, 1, None)));
        assert_eq!(narrow(int(0, 360, 1, Some("No limit")), &range(0, 0), &no_env()), Some(int(0, 0, 1, Some("No limit"))));
    }

    #[test]
    fn extreme_ranges_do_not_overflow() {
        assert_eq!(narrow(int(0, 360, 1, None), &range(i64::MIN, i64::MAX), &no_env()), Some(int(0, 360, 1, None)));
    }

    #[test]
    fn sizes_keep_only_what_the_game_and_the_list_share() {
        let kind = ValueKind::Size { options: vec![Size::new(1280, 720), Size::new(1920, 1080), Size::new(2560, 1440)], native: true };
        let accepted = Constraint::Sizes(vec![Size::new(2560, 1440), Size::new(1920, 1080), Size::new(640, 480)]);
        // Standard order, not the game's; 640x480 is not a listed size; the 2560x1440 screen is accepted.
        let narrowed = narrow(kind, &accepted, &desktop());
        assert_eq!(narrowed, Some(ValueKind::Size { options: vec![Size::new(1920, 1080), Size::new(2560, 1440)], native: true }));
    }

    #[test]
    fn native_needs_a_known_monitor_whose_size_the_game_accepts() {
        let kind = ValueKind::Size { options: vec![Size::new(1280, 720), Size::new(2560, 1440)], native: true };
        let accepted = Constraint::Sizes(vec![Size::new(1280, 720)]);
        assert_eq!(
            narrow(kind.clone(), &accepted, &desktop()),
            Some(ValueKind::Size { options: vec![Size::new(1280, 720)], native: false })
        );
        assert_eq!(
            narrow(kind.clone(), &accepted, &no_env()),
            Some(ValueKind::Size { options: vec![Size::new(1280, 720)], native: false })
        );
        let small = env(DisplayServer::X11, vec![monitor("DP-1", "Small", 1280, 720, true)]);
        assert_eq!(narrow(kind, &accepted, &small), Some(ValueKind::Size { options: vec![Size::new(1280, 720)], native: true }));
    }

    #[test]
    fn sizes_with_nothing_in_common_drop_the_setting() {
        let kind = ValueKind::Size { options: vec![Size::new(1280, 720)], native: true };
        assert_eq!(narrow(kind.clone(), &Constraint::Sizes(vec![Size::new(640, 480)]), &desktop()), None);
        assert_eq!(narrow(kind, &Constraint::Sizes(vec![]), &desktop()), None);
    }

    #[test]
    fn a_native_only_kind_survives_when_the_screen_is_accepted() {
        // A 2560x1600 panel is not in the standard list, but a game that accepts it can still get "native".
        let kind = ValueKind::Size { options: vec![], native: true };
        let panel = env(DisplayServer::X11, vec![monitor("DP-1", "Panel", 2560, 1600, true)]);
        assert_eq!(
            narrow(kind, &Constraint::Sizes(vec![Size::new(2560, 1600)]), &panel),
            Some(ValueKind::Size { options: vec![], native: true })
        );
    }
}
