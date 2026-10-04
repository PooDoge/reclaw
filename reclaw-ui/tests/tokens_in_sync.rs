//! Drift guard: the Rust constants and generated color struct must equal design-system/tokens.json.
//! If this fails, regenerate src/tokens.rs from the design system or fix metrics.rs.
use reclaw_ui::{metrics::*, theme::ThemeKind};
use serde_json::Value;

fn tokens() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../design-system/tokens.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("read tokens.json"))
        .expect("parse tokens.json")
}

fn px(value: &str) -> f32 {
    value
        .trim_end_matches("px")
        .parse()
        .unwrap_or_else(|_| panic!("not a length: {value}"))
}

fn family(t: &Value, name: &str, key: &str) -> f32 {
    t[name]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == key)
        .unwrap_or_else(|| panic!("{name}.{key} missing from tokens.json"))["value"]
        .as_str()
        .map(px)
        .unwrap()
}

#[test]
fn lengths_match() {
    let t = tokens();
    let expected: &[(&str, &str, f32)] = &[
        ("spacing", "space-1", SPACE_1),
        ("spacing", "space-2", SPACE_2),
        ("spacing", "space-3", SPACE_3),
        ("spacing", "space-4", SPACE_4),
        ("spacing", "space-5", SPACE_5),
        ("spacing", "space-6", SPACE_6),
        ("spacing", "space-8", SPACE_8),
        ("radius", "radius-sm", RADIUS_SM),
        ("radius", "radius-md", RADIUS_MD),
        ("radius", "radius-lg", RADIUS_LG),
        ("layout", "sidebar-w", SIDEBAR_W),
        ("layout", "rail-w", RAIL_W),
        ("layout", "topbar-h", TOPBAR_H),
        ("layout", "tabbar-h", TABBAR_H),
        ("layout", "row-h", ROW_H),
        ("layout", "row-h-touch", ROW_H_TOUCH),
        ("layout", "target-min", TARGET_MIN),
        ("layout", "capsule-w", CAPSULE_W),
        ("layout", "capsule-h", CAPSULE_H),
        ("layout", "hero-h", HERO_H),
        ("layout", "bp-wide", BP_WIDE),
        ("layout", "bp-compact", BP_COMPACT),
        ("layout", "deck-safe-x", DECK_SAFE_X),
        ("layout", "deck-safe-y", DECK_SAFE_Y),
        ("layout", "deck-tile-w", DECK_TILE_W),
        ("layout", "deck-tile-h", DECK_TILE_H),
        ("layout", "deck-tile-gap", DECK_TILE_GAP),
        ("layout", "deck-row-h", DECK_ROW_H),
        ("layout", "deck-target-min", DECK_TARGET_MIN),
        ("layout", "deck-hint-h", DECK_HINT_H),
        ("layout", "deck-tabs-h", DECK_TABS_H),
        ("layout", "deck-panel-w", DECK_PANEL_W),
        ("layout", "deck-focus-ring", DECK_FOCUS_RING),
    ];
    for (fam, name, actual) in expected {
        assert_eq!(
            family(&t, fam, name),
            *actual,
            "{fam}.{name} drifted from tokens.json"
        );
    }
    // The scale is a bare number, not px.
    let scale = t["layout"]["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["name"] == "deck-focus-scale")
        .unwrap()["value"]
        .as_str()
        .unwrap()
        .parse::<f32>()
        .unwrap();
    assert_eq!(scale, DECK_FOCUS_SCALE);
}

fn hex(value: &str) -> (u8, u8, u8, u8) {
    let h = value.trim_start_matches('#');
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap();
    (
        byte(0),
        byte(2),
        byte(4),
        if h.len() == 8 { byte(6) } else { 255 },
    )
}

#[test]
fn every_color_token_matches_in_both_themes() {
    let t = tokens();
    let colors = t["color"]["tokens"].as_array().unwrap();
    assert!(colors.len() > 20);
    for (theme, id) in [
        (ThemeKind::Midnight, "midnight"),
        (ThemeKind::Daylight, "daylight"),
    ] {
        let rust = theme.tokens();
        for token in colors {
            let name = token["name"].as_str().unwrap();
            let want = hex(token["value"][id].as_str().unwrap());
            let got = rust
                .get(name)
                .unwrap_or_else(|| panic!("{name} missing from Reclaw::get"));
            assert_eq!(
                (got.r(), got.g(), got.b(), got.a()),
                want,
                "{name} ({id}) drifted from tokens.json"
            );
        }
    }
}

#[test]
fn deck_type_scale_is_what_typography_uses() {
    // The sizes live in typography.rs; this pins them to the design tokens.
    let t = tokens();
    let styles: Vec<&Value> = t["type"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["styles"].as_array().unwrap())
        .collect();
    let size = |name: &str| {
        px(
            styles.iter().find(|s| s["name"] == name).unwrap()["fontSize"]
                .as_str()
                .unwrap(),
        )
    };
    assert_eq!(
        [
            size("deck-title"),
            size("deck-heading"),
            size("deck-body"),
            size("deck-label"),
            size("deck-meta"),
            size("deck-hint")
        ],
        [40., 24., 20., 18., 16., 16.]
    );
}
