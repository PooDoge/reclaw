use super::*;
use crate::settings::{
    capabilities::{Binding, Constraint},
    environment::DisplayServer,
    testing::{args, bind, desktop, env, everything, monitor},
    value::{Choice, Size},
};

fn caps(bindings: Vec<Binding>) -> Capabilities {
    Capabilities { settings: bindings }
}

fn simple(key: SettingKey, constraint: Constraint) -> Binding {
    bind(key, constraint, vec![args(None, &["--x"])])
}

fn layer(items: &[(SettingKey, SettingValue)]) -> SettingsLayer {
    let mut layer = SettingsLayer::default();
    for (key, value) in items {
        layer.set(*key, value.clone());
    }
    layer
}

fn keys(specs: &[SettingSpec]) -> Vec<SettingKey> {
    specs.iter().map(|s| s.key).collect()
}

fn find(items: &[Effective], key: SettingKey) -> &Effective {
    items.iter().find(|e| e.spec.key == key).unwrap_or_else(|| panic!("{key:?} is not offered"))
}

fn two_monitors(server: DisplayServer) -> DisplayEnvironment {
    env(server, vec![monitor("DP-1", "Left", 2560, 1440, true), monitor("HDMI-1", "TV", 3840, 2160, false)])
}

#[test]
fn a_game_that_declares_nothing_shows_nothing_anywhere() {
    for server in [
        DisplayServer::Wayland,
        DisplayServer::X11,
        DisplayServer::Gamescope,
        DisplayServer::Windows,
        DisplayServer::MacOs,
        DisplayServer::Unknown,
    ] {
        let env = two_monitors(server);
        assert_eq!(supported(&Capabilities::default(), &env), vec![]);
        let defaults = layer(&[(SettingKey::Vsync, SettingValue::Bool(false)), (SettingKey::FrameLimit, SettingValue::Int(60))]);
        assert_eq!(effective(&Capabilities::default(), &env, &defaults, &defaults), vec![]);
    }
}

#[test]
fn only_declared_keys_are_supported_in_catalog_order() {
    // Declared out of order on purpose.
    let c = caps(vec![
        simple(SettingKey::Msaa, Constraint::None),
        simple(SettingKey::Vsync, Constraint::None),
        simple(SettingKey::WindowMode, Constraint::None),
    ]);
    let specs = supported(&c, &desktop());
    assert_eq!(keys(&specs), [SettingKey::WindowMode, SettingKey::Vsync, SettingKey::Msaa]);
}

#[test]
fn supported_orders_by_group_then_key() {
    let specs = supported(&everything(), &two_monitors(DisplayServer::X11));
    assert_eq!(keys(&specs), SettingKey::ALL);
    let groups: Vec<_> = specs.iter().map(|s| s.key.group()).collect();
    assert!(groups.windows(2).all(|w| w[0] <= w[1]), "{groups:?}");
}

#[test]
fn a_game_cannot_unlock_what_the_display_refuses() {
    let c = everything();
    let wayland = supported(&c, &two_monitors(DisplayServer::Wayland));
    let Some(ValueKind::Choice(modes)) = wayland.iter().find(|s| s.key == SettingKey::WindowMode).map(|s| s.kind.clone()) else {
        panic!("window mode is offered on Wayland")
    };
    assert_eq!(modes.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["windowed", "borderless"]);

    let gamescope = keys(&supported(&c, &two_monitors(DisplayServer::Gamescope)));
    assert!(!gamescope.contains(&SettingKey::WindowMode) && !gamescope.contains(&SettingKey::Monitor));
    assert_eq!(gamescope.len(), 9);
}

#[test]
fn monitor_needs_two_screens_even_when_declared() {
    assert!(!keys(&supported(&everything(), &desktop())).contains(&SettingKey::Monitor));
    let two = supported(&everything(), &two_monitors(DisplayServer::X11));
    let monitor = two.iter().find(|s| s.key == SettingKey::Monitor).map(|s| s.kind.clone());
    assert_eq!(monitor, Some(ValueKind::Choice(vec![Choice::new("DP-1", "Left (2560x1440)"), Choice::new("HDMI-1", "TV (3840x2160)")])));
}

#[test]
fn constraints_narrow_the_standard_range() {
    let c = caps(vec![
        simple(SettingKey::FrameLimit, Constraint::Range { min: 0, max: 240 }),
        simple(SettingKey::Msaa, Constraint::Choices(vec!["off".into(), "4x".into()])),
        simple(SettingKey::Resolution, Constraint::Sizes(vec![Size::new(1280, 720), Size::new(1920, 1080), Size::new(3840, 2160)])),
    ]);
    let specs = supported(&c, &desktop());
    assert_eq!(specs[0].kind, ValueKind::Size { options: vec![Size::new(1280, 720), Size::new(1920, 1080)], native: false });
    assert_eq!(specs[1].kind, ValueKind::Int { min: 0, max: 240, step: 1, unit: Some("fps"), zero_label: Some("No limit") });
    assert_eq!(specs[2].kind, ValueKind::Choice(vec![Choice::new("off", "Off"), Choice::new("4x", "4x")]));
}

#[test]
fn a_constraint_of_the_wrong_type_does_not_hide_the_setting() {
    let c = caps(vec![
        simple(SettingKey::Vsync, Constraint::Range { min: 0, max: 1 }),
        simple(SettingKey::Msaa, Constraint::Range { min: 0, max: 1 }),
    ]);
    let specs = supported(&c, &desktop());
    assert_eq!(specs[0].kind, ValueKind::Bool);
    assert!(matches!(&specs[1].kind, ValueKind::Choice(options) if options.len() == 4));
}

#[test]
fn an_empty_intersection_drops_the_setting() {
    let c = caps(vec![
        simple(SettingKey::Msaa, Constraint::Choices(vec!["16x".into()])),
        simple(SettingKey::FrameLimit, Constraint::Range { min: 500, max: 600 }),
        simple(SettingKey::Resolution, Constraint::Sizes(vec![Size::new(640, 480)])),
        simple(SettingKey::Vsync, Constraint::None),
    ]);
    assert_eq!(keys(&supported(&c, &desktop())), [SettingKey::Vsync]);
}

#[test]
fn all_specs_lists_what_the_display_can_honor() {
    assert_eq!(all_specs(&DisplayEnvironment::unknown()).len(), 10, "no monitor choice without two screens");
    assert_eq!(all_specs(&two_monitors(DisplayServer::X11)).len(), 11);
    assert_eq!(all_specs(&two_monitors(DisplayServer::Gamescope)).len(), 9);
    assert_eq!(keys(&all_specs(&two_monitors(DisplayServer::X11))), SettingKey::ALL);
}

#[test]
fn nothing_set_means_the_game_decides() {
    let items = effective(&everything(), &desktop(), &SettingsLayer::default(), &SettingsLayer::default());
    assert!(!items.is_empty());
    assert!(items.iter().all(|e| e.value.is_none() && e.source == Source::GameDefault));
}

#[test]
fn override_beats_default_beats_nothing() {
    let c = caps(vec![
        simple(SettingKey::FrameLimit, Constraint::None),
        simple(SettingKey::Msaa, Constraint::None),
        simple(SettingKey::Vsync, Constraint::None),
    ]);
    let defaults = layer(&[(SettingKey::FrameLimit, SettingValue::Int(60)), (SettingKey::Msaa, SettingValue::choice("2x"))]);
    let overrides = layer(&[(SettingKey::FrameLimit, SettingValue::Int(144))]);
    let items = effective(&c, &desktop(), &defaults, &overrides);
    assert_eq!(
        (find(&items, SettingKey::FrameLimit).value.clone(), find(&items, SettingKey::FrameLimit).source),
        (Some(SettingValue::Int(144)), Source::GameOverride)
    );
    assert_eq!(
        (find(&items, SettingKey::Msaa).value.clone(), find(&items, SettingKey::Msaa).source),
        (Some(SettingValue::choice("2x")), Source::UserDefault)
    );
    assert_eq!((find(&items, SettingKey::Vsync).value.clone(), find(&items, SettingKey::Vsync).source), (None, Source::GameDefault));
}

#[test]
fn an_override_that_no_longer_fits_falls_through_to_the_default() {
    let c = caps(vec![simple(SettingKey::Msaa, Constraint::Choices(vec!["off".into(), "2x".into()]))]);
    let defaults = layer(&[(SettingKey::Msaa, SettingValue::choice("2x"))]);
    let stale = layer(&[(SettingKey::Msaa, SettingValue::choice("8x"))]);
    let items = effective(&c, &desktop(), &defaults, &stale);
    assert_eq!(items[0].value, Some(SettingValue::choice("2x")));
    assert_eq!(items[0].source, Source::UserDefault);
}

#[test]
fn a_default_that_does_not_fit_is_skipped_not_replaced() {
    let c = caps(vec![simple(SettingKey::Msaa, Constraint::Choices(vec!["off".into(), "2x".into()]))]);
    let defaults = layer(&[(SettingKey::Msaa, SettingValue::choice("8x"))]);
    let items = effective(&c, &desktop(), &defaults, &SettingsLayer::default());
    assert_eq!((items[0].value.clone(), items[0].source), (None, Source::GameDefault));
}

#[test]
fn a_value_of_the_wrong_type_is_the_same_as_not_set() {
    let c = caps(vec![simple(SettingKey::Vsync, Constraint::None)]);
    let defaults = layer(&[(SettingKey::Vsync, SettingValue::Bool(true))]);
    let wrong = layer(&[(SettingKey::Vsync, SettingValue::Int(0))]);
    let items = effective(&c, &desktop(), &defaults, &wrong);
    assert_eq!((items[0].value.clone(), items[0].source), (Some(SettingValue::Bool(true)), Source::UserDefault));
}

#[test]
fn defaults_are_adapted_to_the_game() {
    let c = caps(vec![
        simple(SettingKey::FrameLimit, Constraint::Range { min: 0, max: 240 }),
        simple(SettingKey::Resolution, Constraint::Sizes(vec![Size::new(1280, 720), Size::new(1920, 1080)])),
    ]);
    let defaults =
        layer(&[(SettingKey::FrameLimit, SettingValue::Int(300)), (SettingKey::Resolution, SettingValue::Size(Size::new(2560, 1440)))]);
    let items = effective(&c, &desktop(), &defaults, &SettingsLayer::default());
    assert_eq!(find(&items, SettingKey::FrameLimit).value, Some(SettingValue::Int(240)));
    assert_eq!(find(&items, SettingKey::Resolution).value, Some(SettingValue::Size(Size::new(1920, 1080))));
}

#[test]
fn layers_cannot_switch_on_what_the_game_does_not_declare() {
    let c = caps(vec![simple(SettingKey::Vsync, Constraint::None)]);
    let all = layer(&[
        (SettingKey::Vsync, SettingValue::Bool(false)),
        (SettingKey::Msaa, SettingValue::choice("4x")),
        (SettingKey::FrameLimit, SettingValue::Int(30)),
    ]);
    let items = effective(&c, &desktop(), &all, &all);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].spec.key, SettingKey::Vsync);
}

#[test]
fn a_monitor_that_was_unplugged_falls_back_to_the_default_monitor() {
    let c = caps(vec![simple(SettingKey::Monitor, Constraint::None)]);
    let env = two_monitors(DisplayServer::X11);
    let defaults = layer(&[(SettingKey::Monitor, SettingValue::choice("HDMI-1"))]);
    let gone = layer(&[(SettingKey::Monitor, SettingValue::choice("DP-9"))]);
    let items = effective(&c, &env, &defaults, &gone);
    assert_eq!((items[0].value.clone(), items[0].source), (Some(SettingValue::choice("HDMI-1")), Source::UserDefault));
}

#[test]
fn an_unsupported_display_hides_a_stored_choice() {
    let c = caps(vec![simple(SettingKey::WindowMode, Constraint::None)]);
    let stored = layer(&[(SettingKey::WindowMode, SettingValue::choice("exclusive"))]);
    assert_eq!(effective(&c, &env(DisplayServer::Gamescope, vec![]), &stored, &stored), vec![]);
    // On Wayland the setting stays, but exclusive no longer fits and nothing replaces it.
    let items = effective(&c, &env(DisplayServer::Wayland, vec![]), &stored, &stored);
    assert_eq!((items[0].value.clone(), items[0].source), (None, Source::GameDefault));
}

#[test]
fn effective_is_stable_when_asked_twice() {
    let c = everything();
    let env = two_monitors(DisplayServer::X11);
    let defaults = layer(&[(SettingKey::FrameLimit, SettingValue::Int(61)), (SettingKey::RenderScale, SettingValue::Int(77))]);
    let first = effective(&c, &env, &defaults, &SettingsLayer::default());
    // Feeding the adapted values back in as the layer changes nothing.
    let adapted = layer(&first.iter().filter_map(|e| Some((e.spec.key, e.value.clone()?))).collect::<Vec<_>>());
    assert_eq!(effective(&c, &env, &adapted, &SettingsLayer::default()), first);
}
