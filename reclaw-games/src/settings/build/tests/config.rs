use super::*;

#[test]
fn environment_variables_are_templated_and_set_once() {
    let c = caps(vec![
        bind(SettingKey::FrameLimit, Constraint::None, vec![env_var("FPS_LIMIT", "{value}"), env_var("SHARED", "from-limit")]),
        bind(
            SettingKey::Msaa,
            Constraint::None,
            vec![env_var("SHARED", "from-msaa-{value}"), env_var("", "ignored"), env_var("ONLY_WIDTH", "{width}")],
        ),
    ]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::FrameLimit, SettingValue::Int(90)), (SettingKey::Msaa, SettingValue::choice("2x"))]);
    assert_eq!(p.env, [("FPS_LIMIT".to_string(), "90".to_string()), ("SHARED".to_string(), "from-msaa-2x".to_string())]);
    assert!(p.args.is_empty() && p.config_edits.is_empty());
}

#[test]
fn config_edits_are_grouped_per_file_in_first_seen_order() {
    let c = caps(vec![
        bind(
            SettingKey::Resolution,
            Constraint::None,
            vec![
                config("a.json", ConfigFormat::Json, "W", "{width}", ValueType::Int),
                config("b.ini", ConfigFormat::Ini, "video.w", "{width}", ValueType::Auto),
                config("a.json", ConfigFormat::Json, "H", "{height}", ValueType::Int),
            ],
        ),
        bind(SettingKey::Vsync, Constraint::None, vec![config("a.json", ConfigFormat::Json, "VSync", "{value}", ValueType::Auto)]),
    ]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::Resolution, size(1920, 1080)), (SettingKey::Vsync, SettingValue::Bool(true))]);
    assert_eq!(p.config_edits.len(), 2);
    assert_eq!(p.config_edits[0].file.relative, "a.json");
    assert_eq!(p.config_edits[0].format, ConfigFormat::Json);
    assert_eq!(
        p.config_edits[0].edits,
        [edit("W", ConfigValue::Int(1920)), edit("H", ConfigValue::Int(1080)), edit("VSync", ConfigValue::Bool(true))]
    );
    assert_eq!(p.config_edits[1].file.relative, "b.ini");
    assert_eq!(p.config_edits[1].edits, [edit("video.w", ConfigValue::Int(1920))]);
}

#[test]
fn a_later_edit_to_the_same_path_wins_in_place() {
    let c = caps(vec![
        bind(
            SettingKey::Vsync,
            Constraint::None,
            vec![
                config("a.json", ConfigFormat::Json, "X", "first", ValueType::String),
                config("a.json", ConfigFormat::Json, "Y", "y", ValueType::String),
            ],
        ),
        bind(SettingKey::Msaa, Constraint::None, vec![config("a.json", ConfigFormat::Json, "X", "{value}", ValueType::String)]),
    ]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::Vsync, SettingValue::Bool(true)), (SettingKey::Msaa, SettingValue::choice("8x"))]);
    assert_eq!(p.config_edits.len(), 1);
    assert_eq!(p.config_edits[0].edits, [edit("X", ConfigValue::Text("8x".into())), edit("Y", ConfigValue::Text("y".into()))]);
}

#[test]
fn the_same_path_in_another_base_or_format_is_another_file() {
    let mut other_base = config("a.json", ConfigFormat::Json, "X", "1", ValueType::Auto);
    if let Target::Config { file, .. } = &mut other_base.target {
        file.base = crate::settings::capabilities::Base::Data;
    }
    let c = caps(vec![bind(
        SettingKey::Vsync,
        Constraint::None,
        vec![
            config("a.json", ConfigFormat::Json, "X", "1", ValueType::Auto),
            other_base,
            config("a.json", ConfigFormat::Toml, "X", "1", ValueType::Auto),
        ],
    )]);
    assert_eq!(plan_of(&c, &desktop(), &[(SettingKey::Vsync, SettingValue::Bool(true))]).config_edits.len(), 3);
}

#[test]
fn config_values_follow_their_type() {
    let c = caps(vec![bind(
        SettingKey::FrameLimit,
        Constraint::None,
        vec![
            config("c.json", ConfigFormat::Json, "auto", "{value}", ValueType::Auto),
            config("c.json", ConfigFormat::Json, "text", "{value}", ValueType::String),
            config("c.json", ConfigFormat::Json, "int", "{value}", ValueType::Int),
            config("c.json", ConfigFormat::Json, "float", "{value}", ValueType::Float),
            config("c.json", ConfigFormat::Json, "bool", "{value}", ValueType::Bool),
            config("c.json", ConfigFormat::Json, "static", "true", ValueType::Auto),
        ],
    )]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::FrameLimit, SettingValue::Int(120))]);
    let got: Vec<_> = p.config_edits[0].edits.iter().map(|e| (e.path.as_str(), e.value.clone())).collect();
    assert_eq!(
        got,
        [
            ("auto", ConfigValue::Int(120)),
            ("text", ConfigValue::Text("120".into())),
            ("int", ConfigValue::Int(120)),
            ("float", ConfigValue::Float(120.0)),
            // "120" is not a boolean, so that edit is skipped rather than guessed.
            ("static", ConfigValue::Bool(true)),
        ]
    );
}

#[test]
fn a_value_that_cannot_take_its_type_is_skipped_with_the_rest_kept() {
    let c = caps(vec![bind(
        SettingKey::UpscaleMethod,
        Constraint::None,
        vec![
            config("c.json", ConfigFormat::Json, "n", "{value}", ValueType::Int),
            config("c.json", ConfigFormat::Json, "name", "{value}", ValueType::String),
        ],
    )]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::UpscaleMethod, SettingValue::choice("fsr1"))]);
    assert_eq!(p.config_edits[0].edits, [edit("name", ConfigValue::Text("fsr1".into()))]);

    let only_bad = caps(vec![bind(
        SettingKey::UpscaleMethod,
        Constraint::None,
        vec![config("c.json", ConfigFormat::Json, "n", "{value}", ValueType::Int)],
    )]);
    assert!(
        plan_of(&only_bad, &desktop(), &[(SettingKey::UpscaleMethod, SettingValue::choice("fsr1"))]).is_empty(),
        "no empty file edit is left behind"
    );
}

#[test]
fn mapped_config_values() {
    let out = with_map(config("c.ini", ConfigFormat::Ini, "gfx.aa", "{mapped}", ValueType::Auto), &[("off", "0"), ("4x", "4")]);
    let c = caps(vec![bind(SettingKey::Msaa, Constraint::None, vec![out])]);
    let p = plan_of(&c, &desktop(), &[(SettingKey::Msaa, SettingValue::choice("4x"))]);
    assert_eq!(p.config_edits[0].edits, [edit("gfx.aa", ConfigValue::Int(4))]);
    assert!(plan_of(&c, &desktop(), &[(SettingKey::Msaa, SettingValue::choice("8x"))]).is_empty());
}
