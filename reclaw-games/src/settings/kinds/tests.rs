use super::*;
use crate::settings::testing::{desktop, env, monitor};

fn ids(kind: Option<ValueKind>) -> Vec<String> {
    match kind {
        Some(ValueKind::Choice(options)) => options.into_iter().map(|c| c.id).collect(),
        other => panic!("expected choices, got {other:?}"),
    }
}

fn options(kind: Option<ValueKind>) -> Vec<Size> {
    match kind {
        Some(ValueKind::Size { options, native }) => {
            assert!(native, "a resolution always offers native");
            options
        }
        other => panic!("expected sizes, got {other:?}"),
    }
}

#[test]
fn window_mode_follows_the_display_server() {
    let kind = |server| standard_kind(SettingKey::WindowMode, &env(server, vec![]));
    assert_eq!(ids(kind(DisplayServer::X11)), ["windowed", "borderless", "exclusive"]);
    assert_eq!(ids(kind(DisplayServer::Windows)), ["windowed", "borderless", "exclusive"]);
    assert_eq!(ids(kind(DisplayServer::MacOs)), ["windowed", "borderless", "exclusive"]);
    assert_eq!(ids(kind(DisplayServer::Unknown)), ["windowed", "borderless", "exclusive"]);
    assert_eq!(ids(kind(DisplayServer::Wayland)), ["windowed", "borderless"]);
    assert_eq!(kind(DisplayServer::Gamescope), None);
}

#[test]
fn resolution_is_the_standard_list_when_no_monitor_is_known() {
    let sizes = options(standard_kind(SettingKey::Resolution, &DisplayEnvironment::unknown()));
    assert_eq!(sizes, STANDARD_SIZES.to_vec());
    assert_eq!(sizes.len(), 8);
    assert_eq!(sizes[0], Size::new(1280, 720));
    assert_eq!(sizes[7], Size::new(3840, 2160));
}

#[test]
fn resolution_stops_at_the_primary_monitors_native_size() {
    let sizes = options(standard_kind(SettingKey::Resolution, &desktop()));
    assert_eq!(sizes.last(), Some(&Size::new(2560, 1440)));
    assert!(!sizes.contains(&Size::new(3440, 1440)) && !sizes.contains(&Size::new(3840, 2160)));

    // Both sides must fit: 2560x1080 is shorter than a 1920x1200 panel but wider.
    let wuxga = env(DisplayServer::X11, vec![monitor("DP-1", "Office", 1920, 1200, true)]);
    assert_eq!(
        options(standard_kind(SettingKey::Resolution, &wuxga)),
        [Size::new(1280, 720), Size::new(1366, 768), Size::new(1600, 900), Size::new(1920, 1080)]
    );

    let ultrawide = env(DisplayServer::X11, vec![monitor("DP-1", "Wide", 3440, 1440, true)]);
    assert!(options(standard_kind(SettingKey::Resolution, &ultrawide)).contains(&Size::new(3440, 1440)));
}

#[test]
fn resolution_uses_the_primary_monitor_not_the_first() {
    let two = env(DisplayServer::X11, vec![monitor("HDMI-1", "TV", 3840, 2160, false), monitor("DP-1", "Small", 1366, 768, true)]);
    assert_eq!(options(standard_kind(SettingKey::Resolution, &two)), [Size::new(1280, 720), Size::new(1366, 768)]);
}

#[test]
fn a_tiny_monitor_still_offers_native() {
    let tiny = env(DisplayServer::X11, vec![monitor("eDP-1", "Handheld", 800, 480, true)]);
    assert!(options(standard_kind(SettingKey::Resolution, &tiny)).is_empty());
}

#[test]
fn monitor_needs_two_screens_and_no_gamescope() {
    let one = desktop();
    assert_eq!(standard_kind(SettingKey::Monitor, &one), None);
    assert_eq!(standard_kind(SettingKey::Monitor, &DisplayEnvironment::unknown()), None);

    let monitors = vec![monitor("DP-1", "Left", 2560, 1440, true), monitor("HDMI-1", "TV", 3840, 2160, false)];
    let two = env(DisplayServer::Wayland, monitors.clone());
    let Some(ValueKind::Choice(choices)) = standard_kind(SettingKey::Monitor, &two) else { panic!("two screens offer a choice") };
    assert_eq!(choices, [Choice::new("DP-1", "Left (2560x1440)"), Choice::new("HDMI-1", "TV (3840x2160)")]);

    assert_eq!(standard_kind(SettingKey::Monitor, &env(DisplayServer::Gamescope, monitors)), None);
}

#[test]
fn choice_catalogs_have_the_documented_ids() {
    let env = DisplayEnvironment::unknown();
    assert_eq!(ids(standard_kind(SettingKey::AspectRatio, &env)), ["auto", "4:3", "16:9", "16:10", "21:9", "32:9"]);
    assert_eq!(ids(standard_kind(SettingKey::UpscaleMethod, &env)), ["off", "linear", "integer", "fsr1"]);
    assert_eq!(ids(standard_kind(SettingKey::Msaa, &env)), ["off", "2x", "4x", "8x"]);
    assert_eq!(ids(standard_kind(SettingKey::TextureFilter, &env)), ["nearest", "linear", "trilinear", "aniso4", "aniso16"]);
}

#[test]
fn numeric_and_boolean_shapes() {
    let env = DisplayEnvironment::unknown();
    assert_eq!(standard_kind(SettingKey::Vsync, &env), Some(ValueKind::Bool));
    assert_eq!(
        standard_kind(SettingKey::FrameLimit, &env),
        Some(ValueKind::Int { min: 0, max: 360, step: 1, unit: Some("fps"), zero_label: Some("No limit") })
    );
    assert_eq!(
        standard_kind(SettingKey::RenderScale, &env),
        Some(ValueKind::Int { min: 25, max: 200, step: 5, unit: Some("%"), zero_label: None })
    );
    assert_eq!(
        standard_kind(SettingKey::Sharpness, &env),
        Some(ValueKind::Int { min: 0, max: 100, step: 5, unit: Some("%"), zero_label: None })
    );
}

#[test]
fn every_key_has_an_answer_on_every_display_server() {
    let servers = [
        DisplayServer::Wayland,
        DisplayServer::X11,
        DisplayServer::Gamescope,
        DisplayServer::Windows,
        DisplayServer::MacOs,
        DisplayServer::Unknown,
    ];
    for server in servers {
        for key in SettingKey::ALL {
            let kind = standard_kind(key, &env(server, vec![]));
            let hidden = matches!(key, SettingKey::Monitor) || (key == SettingKey::WindowMode && server == DisplayServer::Gamescope);
            assert_eq!(kind.is_none(), hidden, "{key:?} on {server:?}");
        }
    }
}

#[test]
fn every_choice_has_a_unique_id_and_a_label() {
    let env = DisplayEnvironment::unknown();
    for key in SettingKey::ALL {
        if let Some(ValueKind::Choice(options)) = standard_kind(key, &env) {
            let mut seen = std::collections::HashSet::new();
            for option in &options {
                assert!(seen.insert(option.id.clone()), "{key:?} repeats {}", option.id);
                assert!(!option.label.is_empty());
            }
        }
    }
}
