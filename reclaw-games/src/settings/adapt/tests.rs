use super::*;
use crate::settings::{
    environment::{DisplayEnvironment, DisplayServer},
    key::SettingKey,
    resolve::all_specs,
    testing::{desktop, env, monitor},
    value::Choice,
};

fn spec(kind: ValueKind) -> SettingSpec {
    SettingSpec { key: SettingKey::Vsync, kind }
}

fn int_spec(min: i64, max: i64, step: i64) -> SettingSpec {
    spec(ValueKind::Int { min, max, step, unit: None, zero_label: None })
}

fn adapt_int(spec: &SettingSpec, v: i64) -> Option<i64> {
    match adapt(spec, &SettingValue::Int(v)) {
        Some(SettingValue::Int(n)) => Some(n),
        Some(other) => panic!("an Int became {other:?}"),
        None => None,
    }
}

fn sizes_spec(options: &[(u32, u32)], native: bool) -> SettingSpec {
    spec(ValueKind::Size { options: options.iter().map(|(w, h)| Size::new(*w, *h)).collect(), native })
}

#[test]
fn bool_takes_only_bool() {
    let s = spec(ValueKind::Bool);
    assert_eq!(adapt(&s, &SettingValue::Bool(false)), Some(SettingValue::Bool(false)));
    assert_eq!(adapt(&s, &SettingValue::Bool(true)), Some(SettingValue::Bool(true)));
    assert_eq!(adapt(&s, &SettingValue::Int(1)), None);
    assert_eq!(adapt(&s, &SettingValue::choice("true")), None);
    assert_eq!(adapt(&s, &SettingValue::Native), None);
}

#[test]
fn int_is_clamped_into_range() {
    let s = int_spec(0, 240, 1);
    assert_eq!(adapt_int(&s, 144), Some(144));
    assert_eq!(adapt_int(&s, 360), Some(240));
    assert_eq!(adapt_int(&s, -1), Some(0));
    assert_eq!(adapt_int(&s, i64::MAX), Some(240));
    assert_eq!(adapt_int(&s, i64::MIN), Some(0));
}

#[test]
fn int_snaps_to_the_nearest_step_from_min() {
    let s = int_spec(25, 200, 5);
    assert_eq!(adapt_int(&s, 100), Some(100));
    assert_eq!(adapt_int(&s, 27), Some(25));
    assert_eq!(adapt_int(&s, 28), Some(30));
    assert_eq!(adapt_int(&s, 10), Some(25), "clamped first");
    assert_eq!(adapt_int(&s, 203), Some(200));
    // The grid starts at min, not at zero: 7 steps of 10 from 3 are 3, 13, 23 ...
    let odd = int_spec(3, 100, 10);
    assert_eq!(adapt_int(&odd, 12), Some(13));
    assert_eq!(adapt_int(&odd, 7), Some(3));
}

#[test]
fn int_halves_round_up() {
    let s = int_spec(0, 100, 10);
    assert_eq!(adapt_int(&s, 4), Some(0));
    assert_eq!(adapt_int(&s, 5), Some(10));
    assert_eq!(adapt_int(&s, 15), Some(20));
}

#[test]
fn int_never_snaps_past_a_max_that_is_off_the_grid() {
    let s = int_spec(0, 8, 5);
    assert_eq!(adapt_int(&s, 8), Some(5));
    assert_eq!(adapt_int(&s, 100), Some(5));
    assert_eq!(adapt_int(&s, 3), Some(5));
}

#[test]
fn int_survives_a_malformed_spec() {
    assert_eq!(adapt_int(&int_spec(10, 5, 1), 7), None, "min above max has no value");
    assert_eq!(adapt_int(&int_spec(0, 10, 0), 7), Some(7), "step 0 acts as 1");
    assert_eq!(adapt_int(&int_spec(0, 10, -3), 7), Some(7), "a negative step acts as 1");
    assert_eq!(adapt_int(&int_spec(i64::MIN, i64::MAX, i64::MAX), 0), Some(-1));
    assert_eq!(adapt_int(&int_spec(i64::MIN, i64::MAX, 1), i64::MAX), Some(i64::MAX));
}

#[test]
fn int_takes_only_int() {
    let s = int_spec(0, 10, 1);
    assert_eq!(adapt(&s, &SettingValue::Bool(true)), None);
    assert_eq!(adapt(&s, &SettingValue::choice("5")), None);
    assert_eq!(adapt(&s, &SettingValue::Size(Size::new(5, 5))), None);
}

#[test]
fn choice_must_be_an_option() {
    let s = spec(ValueKind::Choice(vec![Choice::new("off", "Off"), Choice::new("4x", "4x")]));
    assert_eq!(adapt(&s, &SettingValue::choice("4x")), Some(SettingValue::choice("4x")));
    assert_eq!(adapt(&s, &SettingValue::choice("8x")), None, "not replaced by a neighbour");
    assert_eq!(adapt(&s, &SettingValue::choice("4X")), None);
    assert_eq!(adapt(&s, &SettingValue::Int(4)), None);
}

#[test]
fn a_listed_size_is_kept() {
    let s = sizes_spec(&[(1280, 720), (1920, 1080)], false);
    assert_eq!(adapt(&s, &SettingValue::Size(Size::new(1920, 1080))), Some(SettingValue::Size(Size::new(1920, 1080))));
}

#[test]
fn an_unlisted_size_becomes_the_largest_listed_area_not_above_it() {
    let s = sizes_spec(&[(1280, 720), (1920, 1080), (3840, 2160)], false);
    let fit = |w, h| adapt(&s, &SettingValue::Size(Size::new(w, h)));
    assert_eq!(fit(2560, 1440), Some(SettingValue::Size(Size::new(1920, 1080))));
    assert_eq!(fit(1920, 1081), Some(SettingValue::Size(Size::new(1920, 1080))));
    assert_eq!(fit(5120, 2880), Some(SettingValue::Size(Size::new(3840, 2160))));
    assert_eq!(fit(1280, 721), Some(SettingValue::Size(Size::new(1280, 720))));
    assert_eq!(fit(1000, 700), None, "every listed size is larger than asked");
    assert_eq!(fit(0, 0), None);
}

#[test]
fn size_is_chosen_by_area_in_list_order() {
    // 2560x1080 has the larger area than 1920x1080 and is "not above" 2560x1440.
    let s = sizes_spec(&[(1920, 1080), (2560, 1080)], false);
    assert_eq!(adapt(&s, &SettingValue::Size(Size::new(2560, 1440))), Some(SettingValue::Size(Size::new(2560, 1080))));
    // Equal areas: the first listed wins, whatever the order.
    let tie = sizes_spec(&[(1600, 900), (900, 1600), (800, 800)], false);
    assert_eq!(adapt(&tie, &SettingValue::Size(Size::new(1700, 1000))), Some(SettingValue::Size(Size::new(1600, 900))));
}

#[test]
fn native_needs_a_kind_that_allows_it() {
    assert_eq!(adapt(&sizes_spec(&[(1280, 720)], true), &SettingValue::Native), Some(SettingValue::Native));
    assert_eq!(adapt(&sizes_spec(&[(1280, 720)], false), &SettingValue::Native), None);
    assert_eq!(adapt(&sizes_spec(&[], true), &SettingValue::Native), Some(SettingValue::Native));
    assert_eq!(adapt(&spec(ValueKind::Bool), &SettingValue::Native), None);
}

#[test]
fn a_size_on_an_empty_list_is_nothing() {
    assert_eq!(adapt(&sizes_spec(&[], true), &SettingValue::Size(Size::new(1920, 1080))), None);
}

#[test]
fn adapt_is_idempotent_for_every_standard_spec() {
    let candidates = [
        SettingValue::Bool(true),
        SettingValue::Bool(false),
        SettingValue::Int(-7),
        SettingValue::Int(0),
        SettingValue::Int(28),
        SettingValue::Int(144),
        SettingValue::Int(1000),
        SettingValue::choice("off"),
        SettingValue::choice("4x"),
        SettingValue::choice("windowed"),
        SettingValue::choice("DP-1"),
        SettingValue::Size(Size::new(1920, 1080)),
        SettingValue::Size(Size::new(2000, 1100)),
        SettingValue::Size(Size::new(100, 100)),
        SettingValue::Native,
    ];
    let wayland_pair = env(DisplayServer::Wayland, vec![monitor("DP-1", "A", 2560, 1440, true), monitor("DP-2", "B", 1920, 1080, false)]);
    for env in [DisplayEnvironment::unknown(), desktop(), wayland_pair] {
        for spec in all_specs(&env) {
            for value in &candidates {
                if let Some(once) = adapt(&spec, value) {
                    assert_eq!(adapt(&spec, &once), Some(once.clone()), "{:?} / {value:?}", spec.key);
                }
            }
        }
    }
}

#[test]
fn everything_a_game_offers_is_accepted_as_it_is() {
    use crate::settings::{
        capabilities::{Capabilities, Constraint},
        resolve::supported,
        testing::{bind, everything},
    };
    let narrowed = Capabilities {
        settings: vec![
            bind(SettingKey::RenderScale, Constraint::Range { min: 26, max: 42 }, vec![]),
            bind(SettingKey::FrameLimit, Constraint::Range { min: 30, max: 240 }, vec![]),
            bind(SettingKey::Msaa, Constraint::Choices(vec!["4x".into(), "off".into()]), vec![]),
            bind(
                SettingKey::Resolution,
                Constraint::Sizes(vec![Size::new(1920, 1080), Size::new(2560, 1440), Size::new(3840, 2160)]),
                vec![],
            ),
        ],
    };
    let wayland_pair = env(DisplayServer::Wayland, vec![monitor("DP-1", "A", 2560, 1440, true), monitor("DP-2", "B", 1920, 1080, false)]);
    for caps in [everything(), narrowed] {
        for env in [DisplayEnvironment::unknown(), desktop(), wayland_pair.clone()] {
            for spec in supported(&caps, &env) {
                let own: Vec<SettingValue> = match &spec.kind {
                    ValueKind::Bool => vec![SettingValue::Bool(true), SettingValue::Bool(false)],
                    ValueKind::Int { min, max, .. } => vec![SettingValue::Int(*min), SettingValue::Int(*max)],
                    ValueKind::Choice(options) => options.iter().map(|o| SettingValue::choice(&o.id)).collect(),
                    ValueKind::Size { options, native } => {
                        options.iter().map(|s| SettingValue::Size(*s)).chain(native.then_some(SettingValue::Native)).collect()
                    }
                };
                assert!(!own.is_empty(), "{:?} offers nothing", spec.key);
                for value in own {
                    assert_eq!(adapt(&spec, &value), Some(value.clone()), "{:?}", spec.key);
                }
            }
        }
    }
}
