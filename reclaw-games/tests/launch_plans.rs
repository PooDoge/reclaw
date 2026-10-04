//! The sample games from `reclaw_games::sample`, taken from catalog entry to the exact command line
//! and config edits a launch would use.
use reclaw_games::{
    sample::sample_projects,
    settings::{
        Base, Capabilities, ConfigFormat, ConfigValue, DisplayEnvironment, DisplayServer, KeyEdit, Monitor, SettingKey, SettingValue,
        SettingsLayer, Size, Source, ValueKind, apply_edits, effective, plan, supported,
    },
};

fn capabilities_of(title: &str) -> Capabilities {
    sample_projects().into_iter().find(|p| p.title == title).unwrap_or_else(|| panic!("no sample project {title}")).capabilities
}

fn monitor(id: &str, width: u32, height: u32, primary: bool) -> Monitor {
    Monitor { id: id.into(), name: id.into(), native: Size::new(width, height), refresh_mhz: 144_000, primary }
}

fn screen(server: DisplayServer, width: u32, height: u32) -> DisplayEnvironment {
    DisplayEnvironment { server, monitors: vec![monitor("DP-1", width, height, true)] }
}

fn layer(items: &[(SettingKey, SettingValue)]) -> SettingsLayer {
    let mut layer = SettingsLayer::default();
    for (key, value) in items {
        layer.set(*key, value.clone());
    }
    layer
}

/// The user's global defaults used throughout: borderless, 1440p, vsync off, 144 fps.
fn defaults() -> SettingsLayer {
    layer(&[
        (SettingKey::WindowMode, SettingValue::choice("borderless")),
        (SettingKey::Resolution, SettingValue::Size(Size::new(2560, 1440))),
        (SettingKey::Vsync, SettingValue::Bool(false)),
        (SettingKey::FrameLimit, SettingValue::Int(144)),
    ])
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn starfall_64_gets_exactly_these_arguments() {
    let caps = capabilities_of("Starfall 64");
    let p = plan(&caps, &screen(DisplayServer::X11, 2560, 1440), &defaults(), &SettingsLayer::default());
    assert_eq!(p.args, strings(&["--borderless", "--width", "2560", "--height", "1440", "--no-vsync", "--fps", "144"]));
    assert!(p.env.is_empty() && p.config_edits.is_empty());
}

#[test]
fn starfall_64_offers_its_four_settings_and_nothing_else() {
    let keys: Vec<_> =
        supported(&capabilities_of("Starfall 64"), &screen(DisplayServer::X11, 3840, 2160)).into_iter().map(|s| s.key).collect();
    assert_eq!(keys, [SettingKey::WindowMode, SettingKey::Resolution, SettingKey::Vsync, SettingKey::FrameLimit]);
}

#[test]
fn starfall_64_follows_the_display() {
    let caps = capabilities_of("Starfall 64");
    let exclusive = layer(&[(SettingKey::WindowMode, SettingValue::choice("exclusive")), (SettingKey::Vsync, SettingValue::Bool(true))]);
    let on = |server| plan(&caps, &screen(server, 2560, 1440), &exclusive, &SettingsLayer::default()).args;
    assert_eq!(on(DisplayServer::X11), strings(&["--fullscreen"]));
    assert_eq!(on(DisplayServer::Wayland), Vec::<String>::new(), "Wayland cannot honor exclusive, and vsync on needs no flag");
    assert_eq!(on(DisplayServer::Gamescope), Vec::<String>::new());
}

#[test]
fn starfall_64_uses_the_game_override_and_adapts_the_default() {
    let caps = capabilities_of("Starfall 64");
    let overrides = layer(&[(SettingKey::FrameLimit, SettingValue::Int(60)), (SettingKey::WindowMode, SettingValue::choice("windowed"))]);
    let env = screen(DisplayServer::X11, 1920, 1080);
    let items = effective(&caps, &env, &defaults(), &overrides);
    let source = |key| items.iter().find(|e| e.spec.key == key).map(|e| (e.value.clone(), e.source));
    assert_eq!(source(SettingKey::FrameLimit), Some((Some(SettingValue::Int(60)), Source::GameOverride)));
    assert_eq!(source(SettingKey::WindowMode), Some((Some(SettingValue::choice("windowed")), Source::GameOverride)));
    // 1440p does not fit a 1080p screen: the largest listed size below it is used.
    assert_eq!(source(SettingKey::Resolution), Some((Some(SettingValue::Size(Size::new(1920, 1080))), Source::UserDefault)));
    assert_eq!(source(SettingKey::Vsync), Some((Some(SettingValue::Bool(false)), Source::UserDefault)));

    let p = plan(&caps, &env, &defaults(), &overrides);
    assert_eq!(p.args, strings(&["--windowed", "--width", "1920", "--height", "1080", "--no-vsync", "--fps", "60"]));
}

#[test]
fn starfall_64_clamps_a_frame_limit_the_game_cannot_do() {
    let caps = capabilities_of("Starfall 64");
    let fast = layer(&[(SettingKey::FrameLimit, SettingValue::Int(360))]);
    assert_eq!(plan(&caps, &screen(DisplayServer::X11, 2560, 1440), &fast, &SettingsLayer::default()).args, strings(&["--fps", "240"]));
}

#[test]
fn skyward_quest_writes_these_json_edits() {
    let caps = capabilities_of("Skyward Quest");
    let p = plan(&caps, &screen(DisplayServer::X11, 2560, 1440), &defaults(), &SettingsLayer::default());
    assert!(p.args.is_empty() && p.env.is_empty(), "it reads a config file, not the command line: {p:?}");
    assert_eq!(p.config_edits.len(), 1);
    let file = &p.config_edits[0];
    assert_eq!((file.file.base, file.file.relative.as_str(), file.format), (Base::Config, "graphics.json", ConfigFormat::Json));
    assert_eq!(
        file.edits,
        [
            KeyEdit { path: "Graphics.Width".into(), value: ConfigValue::Int(2560) },
            KeyEdit { path: "Graphics.Height".into(), value: ConfigValue::Int(1440) }
        ]
    );
}

#[test]
fn skyward_quest_does_not_show_what_it_cannot_apply() {
    let keys: Vec<_> =
        supported(&capabilities_of("Skyward Quest"), &screen(DisplayServer::X11, 3840, 2160)).into_iter().map(|s| s.key).collect();
    // The user's window mode, vsync and frame limit defaults have nothing to attach to here.
    assert_eq!(keys, [SettingKey::Resolution, SettingKey::AspectRatio, SettingKey::UpscaleMethod, SettingKey::Msaa]);
}

#[test]
fn skyward_quest_edits_the_game_file_and_keeps_the_rest() {
    let caps = capabilities_of("Skyward Quest");
    let overrides = layer(&[
        (SettingKey::Msaa, SettingValue::choice("4x")),
        (SettingKey::UpscaleMethod, SettingValue::choice("fsr1")),
        (SettingKey::AspectRatio, SettingValue::choice("16:9")),
    ]);
    let p = plan(&caps, &screen(DisplayServer::X11, 2560, 1440), &defaults(), &overrides);
    let file = &p.config_edits[0];
    assert_eq!(
        file.edits.iter().map(|e| (e.path.as_str(), e.value.clone())).collect::<Vec<_>>(),
        [
            ("Graphics.Width", ConfigValue::Int(2560)),
            ("Graphics.Height", ConfigValue::Int(1440)),
            ("Graphics.AspectRatio", ConfigValue::Text("16:9".into())),
            ("Graphics.Upscaler", ConfigValue::Text("fsr1".into())),
            ("Graphics.MSAA", ConfigValue::Text("4x".into())),
        ]
    );

    let before = "{\n  \"Version\": 3,\n  \"Graphics\": {\n    \"MSAA\": \"off\",\n    \"Width\": 1280,\n    \"Height\": 720\n  },\n  \"Audio\": { \"Volume\": 7 }\n}\n";
    let after = apply_edits(file.format, before, &file.edits).expect("the sample file edits cleanly");
    assert_eq!(
        after,
        "{\n  \"Version\": 3,\n  \"Graphics\": {\n    \"MSAA\": \"4x\",\n    \"Width\": 2560,\n    \"Height\": 1440,\n    \"AspectRatio\": \"16:9\",\n    \"Upscaler\": \"fsr1\"\n  },\n  \"Audio\": {\n    \"Volume\": 7\n  }\n}\n"
    );
}

#[test]
fn skyward_quest_native_needs_a_screen_it_accepts() {
    let caps = capabilities_of("Skyward Quest");
    let native = layer(&[(SettingKey::Resolution, SettingValue::Native)]);
    let on = |w, h| plan(&caps, &screen(DisplayServer::X11, w, h), &native, &SettingsLayer::default());
    assert_eq!(on(3840, 2160).config_edits[0].edits[0].value, ConfigValue::Int(3840));
    assert!(on(3440, 1440).is_empty(), "3440x1440 is not a size this game accepts, so native is not offered");
}

#[test]
fn skyward_quest_lowers_a_resolution_that_is_not_listed_for_the_game() {
    let caps = capabilities_of("Skyward Quest");
    let odd = layer(&[(SettingKey::Resolution, SettingValue::Size(Size::new(1600, 900)))]);
    let p = plan(&caps, &screen(DisplayServer::X11, 3840, 2160), &odd, &SettingsLayer::default());
    assert_eq!(p.config_edits[0].edits[0].value, ConfigValue::Int(1280), "the game lists 1280x720 below 1600x900");
}

#[test]
fn a_game_with_no_capabilities_shows_and_applies_nothing() {
    let caps = capabilities_of("Kart Ruins");
    assert!(caps.is_empty());
    let two_screens = DisplayEnvironment {
        server: DisplayServer::Wayland,
        monitors: vec![monitor("DP-1", 2560, 1440, true), monitor("DP-2", 1920, 1080, false)],
    };
    for env in [DisplayEnvironment::unknown(), screen(DisplayServer::X11, 2560, 1440), two_screens] {
        assert_eq!(supported(&caps, &env), vec![]);
        assert!(plan(&caps, &env, &defaults(), &defaults()).is_empty());
    }
}

#[test]
fn empty_layers_plan_nothing_for_any_sample_game() {
    for project in sample_projects() {
        let p = plan(&project.capabilities, &screen(DisplayServer::X11, 2560, 1440), &SettingsLayer::default(), &SettingsLayer::default());
        assert!(p.is_empty(), "{}: {p:?}", project.title);
    }
}

#[test]
fn every_effective_setting_is_one_the_game_declares() {
    let everything = layer(&[
        (SettingKey::WindowMode, SettingValue::choice("windowed")),
        (SettingKey::Resolution, SettingValue::Native),
        (SettingKey::AspectRatio, SettingValue::choice("auto")),
        (SettingKey::Vsync, SettingValue::Bool(true)),
        (SettingKey::FrameLimit, SettingValue::Int(30)),
        (SettingKey::UpscaleMethod, SettingValue::choice("off")),
        (SettingKey::RenderScale, SettingValue::Int(100)),
        (SettingKey::Sharpness, SettingValue::Int(50)),
        (SettingKey::Msaa, SettingValue::choice("off")),
        (SettingKey::TextureFilter, SettingValue::choice("linear")),
    ]);
    for project in sample_projects() {
        for env in [DisplayEnvironment::unknown(), screen(DisplayServer::Gamescope, 1280, 800), screen(DisplayServer::Wayland, 3840, 2160)]
        {
            for item in effective(&project.capabilities, &env, &everything, &SettingsLayer::default()) {
                assert!(project.capabilities.supports(item.spec.key), "{} shows undeclared {:?}", project.title, item.spec.key);
                if let ValueKind::Choice(options) = &item.spec.kind {
                    assert!(!options.is_empty());
                }
            }
        }
    }
}
