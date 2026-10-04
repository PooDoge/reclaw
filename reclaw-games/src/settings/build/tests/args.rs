use super::*;

#[test]
fn nothing_chosen_means_an_empty_plan() {
    let p = plan(&everything(), &two_monitors(DisplayServer::X11), &SettingsLayer::default(), &SettingsLayer::default());
    assert!(p.is_empty(), "{p:?}");
}

#[test]
fn a_game_with_no_capabilities_gets_nothing_whatever_the_user_chose() {
    let everything_chosen = layer(&[
        (SettingKey::Vsync, SettingValue::Bool(false)),
        (SettingKey::FrameLimit, SettingValue::Int(60)),
        (SettingKey::Resolution, SettingValue::Native),
    ]);
    assert!(plan(&Capabilities::default(), &desktop(), &everything_chosen, &everything_chosen).is_empty());
}

#[test]
fn arguments_follow_the_catalog_order_not_the_declaration_order() {
    let c = caps(vec![
        bind(SettingKey::Msaa, Constraint::None, vec![args(None, &["--msaa", "{value}"])]),
        bind(SettingKey::Vsync, Constraint::None, vec![args(None, &["--vsync", "{value}"])]),
        bind(SettingKey::WindowMode, Constraint::None, vec![args(None, &["--mode", "{value}"])]),
    ]);
    let p = plan_of(
        &c,
        &desktop(),
        &[
            (SettingKey::Msaa, SettingValue::choice("4x")),
            (SettingKey::Vsync, SettingValue::Bool(true)),
            (SettingKey::WindowMode, SettingValue::choice("windowed")),
        ],
    );
    assert_eq!(p.args, strings(&["--mode", "windowed", "--vsync", "true", "--msaa", "4x"]));
}

#[test]
fn a_binding_runs_every_output_in_order() {
    let c = caps(vec![bind(
        SettingKey::FrameLimit,
        Constraint::None,
        vec![args(None, &["--a"]), args(None, &["--b", "{value}"]), args(None, &["--c"])],
    )]);
    assert_eq!(plan_of(&c, &desktop(), &[(SettingKey::FrameLimit, SettingValue::Int(30))]).args, strings(&["--a", "--b", "30", "--c"]));
}

#[test]
fn when_picks_the_output_for_the_chosen_value() {
    let c = caps(vec![bind(
        SettingKey::WindowMode,
        Constraint::None,
        vec![
            args(Some(SettingValue::choice("exclusive")), &["--fullscreen"]),
            args(Some(SettingValue::choice("borderless")), &["--borderless"]),
            args(Some(SettingValue::choice("windowed")), &["--windowed"]),
        ],
    )]);
    for (mode, expected) in [("exclusive", "--fullscreen"), ("borderless", "--borderless"), ("windowed", "--windowed")] {
        assert_eq!(plan_of(&c, &desktop(), &[(SettingKey::WindowMode, SettingValue::choice(mode))]).args, strings(&[expected]), "{mode}");
    }
}

#[test]
fn a_boolean_when_emits_only_for_that_side() {
    let c = caps(vec![bind(SettingKey::Vsync, Constraint::None, vec![args(Some(SettingValue::Bool(false)), &["--no-vsync"])])]);
    assert_eq!(plan_of(&c, &desktop(), &[(SettingKey::Vsync, SettingValue::Bool(false))]).args, strings(&["--no-vsync"]));
    assert!(plan_of(&c, &desktop(), &[(SettingKey::Vsync, SettingValue::Bool(true))]).is_empty());
}

#[test]
fn an_argument_list_is_all_or_nothing() {
    let c = caps(vec![bind(
        SettingKey::Msaa,
        Constraint::None,
        vec![with_map(args(None, &["--aa", "{mapped}", "--aa-on"]), &[("4x", "MSAA_4")])],
    )]);
    assert_eq!(plan_of(&c, &desktop(), &[(SettingKey::Msaa, SettingValue::choice("4x"))]).args, strings(&["--aa", "MSAA_4", "--aa-on"]));
    assert!(
        plan_of(&c, &desktop(), &[(SettingKey::Msaa, SettingValue::choice("2x"))]).is_empty(),
        "no map entry: not even the flag that needs none"
    );

    let half = caps(vec![bind(SettingKey::FrameLimit, Constraint::None, vec![args(None, &["--first", "--w", "{width}"])])]);
    assert!(plan_of(&half, &desktop(), &[(SettingKey::FrameLimit, SettingValue::Int(30))]).is_empty());
}

#[test]
fn a_size_fills_width_and_height() {
    let p = plan_of(&resolution_args(), &desktop(), &[(SettingKey::Resolution, size(1920, 1080))]);
    assert_eq!(p.args, strings(&["--width", "1920", "--height", "1080"]));
}

#[test]
fn the_override_wins_in_the_plan_and_an_invalid_one_falls_through() {
    let c = caps(vec![
        bind(SettingKey::FrameLimit, Constraint::Range { min: 0, max: 144 }, vec![args(None, &["--fps", "{value}"])]),
        bind(SettingKey::Msaa, Constraint::Choices(vec!["off".into(), "2x".into()]), vec![args(None, &["--msaa", "{value}"])]),
    ]);
    let defaults = layer(&[(SettingKey::FrameLimit, SettingValue::Int(60)), (SettingKey::Msaa, SettingValue::choice("2x"))]);
    let overrides = layer(&[(SettingKey::FrameLimit, SettingValue::Int(100)), (SettingKey::Msaa, SettingValue::choice("8x"))]);
    assert_eq!(plan(&c, &desktop(), &defaults, &overrides).args, strings(&["--fps", "100", "--msaa", "2x"]));
    assert_eq!(plan(&c, &desktop(), &defaults, &SettingsLayer::default()).args, strings(&["--fps", "60", "--msaa", "2x"]));
}

#[test]
fn wayland_never_gets_an_exclusive_fullscreen_argument() {
    let c = caps(vec![bind(
        SettingKey::WindowMode,
        Constraint::None,
        vec![
            args(Some(SettingValue::choice("exclusive")), &["--fullscreen"]),
            args(Some(SettingValue::choice("borderless")), &["--borderless"]),
        ],
    )]);
    let chosen = [(SettingKey::WindowMode, SettingValue::choice("exclusive"))];
    assert_eq!(plan_of(&c, &env(DisplayServer::X11, vec![]), &chosen).args, strings(&["--fullscreen"]));
    assert!(plan_of(&c, &env(DisplayServer::Wayland, vec![]), &chosen).is_empty());
    assert!(
        plan_of(&c, &env(DisplayServer::Gamescope, vec![]), &[(SettingKey::WindowMode, SettingValue::choice("borderless"))]).is_empty()
    );
}

#[test]
fn gamescope_gets_no_monitor_or_window_mode_even_if_declared_and_chosen() {
    let chosen = [
        (SettingKey::WindowMode, SettingValue::choice("borderless")),
        (SettingKey::Monitor, SettingValue::choice("HDMI-1")),
        (SettingKey::Vsync, SettingValue::Bool(false)),
    ];
    let p = plan_of(&everything(), &two_monitors(DisplayServer::Gamescope), &chosen);
    assert_eq!(p.args, strings(&["--display.vsync=false"]));
}
