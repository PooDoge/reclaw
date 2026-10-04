use std::fs;

use super::*;
use crate::settings::{
    capabilities::{ConfigFormat, ConfigPath},
    testing::TempDir,
};

fn bases(dir: &TempDir) -> Bases {
    Bases { install: dir.path().join("install"), config: dir.path().join("config"), data: dir.path().join("data") }
}

fn at(base: Base, relative: &str) -> ConfigPath {
    ConfigPath { base, relative: relative.to_string() }
}

fn set(path: &str, value: ConfigValue) -> KeyEdit {
    KeyEdit { path: path.to_string(), value }
}

fn file_edit(base: Base, relative: &str, format: ConfigFormat, edits: Vec<KeyEdit>) -> ConfigFileEdit {
    ConfigFileEdit { file: at(base, relative), format, edits }
}

fn plan_of(config_edits: Vec<ConfigFileEdit>) -> LaunchPlan {
    LaunchPlan { config_edits, ..LaunchPlan::default() }
}

#[test]
fn resolve_joins_the_right_base() {
    let b = Bases { install: "/games/x".into(), config: "/home/u/.config/x".into(), data: "/home/u/.local/share/x".into() };
    assert_eq!(b.resolve(&at(Base::Install, "game.ini")).expect("ok"), PathBuf::from("/games/x/game.ini"));
    assert_eq!(b.resolve(&at(Base::Config, "game.ini")).expect("ok"), PathBuf::from("/home/u/.config/x/game.ini"));
    assert_eq!(b.resolve(&at(Base::Data, "saves/settings.json")).expect("ok"), PathBuf::from("/home/u/.local/share/x/saves/settings.json"));
}

#[test]
fn resolve_accepts_harmless_spellings() {
    let b = Bases { install: "/g".into(), config: "/c".into(), data: "/d".into() };
    for (relative, expected) in [
        ("./a.json", "/g/a.json"),
        ("a//b.json", "/g/a/b.json"),
        ("a\\b.json", "/g/a/b.json"),
        ("a/./b", "/g/a/b"),
        ("..hidden", "/g/..hidden"),
        ("a..b/c..", "/g/a..b/c.."),
        ("sub/", "/g/sub"),
    ] {
        assert_eq!(b.resolve(&at(Base::Install, relative)).expect(relative), PathBuf::from(expected), "{relative}");
    }
}

#[test]
fn resolve_refuses_anything_that_leaves_the_base() {
    let b = Bases { install: "/g".into(), config: "/c".into(), data: "/d".into() };
    for relative in [
        "/etc/passwd",
        "\\windows\\system32",
        "C:\\Windows\\x.ini",
        "c:/x",
        "C:x",
        "..",
        "../x",
        "a/../b",
        "a/..",
        "a\\..\\b",
        "a/b/../../..",
        "./../x",
        "a/../../x",
        "//share/x",
    ] {
        let err = b.resolve(&at(Base::Install, relative)).expect_err(relative);
        assert!(matches!(err, ConfigEditError::EscapesBase(ref p) if p == relative), "{relative}: {err}");
    }
}

#[test]
fn resolve_refuses_a_path_that_names_nothing() {
    let b = Bases { install: "/g".into(), config: "/c".into(), data: "/d".into() };
    for relative in ["", "  ", ".", "./", "./."] {
        assert!(matches!(b.resolve(&at(Base::Install, relative)), Err(ConfigEditError::BadPath { .. })), "{relative:?}");
    }
}

#[test]
fn an_empty_plan_writes_nothing() {
    let dir = TempDir::new();
    assert_eq!(LaunchPlan::default().apply_config(&bases(&dir)).expect("ok"), Vec::<PathBuf>::new());
    assert!(!dir.path().join("config").exists());
}

#[test]
fn apply_config_edits_an_existing_file_and_keeps_the_original() {
    let dir = TempDir::new();
    let b = bases(&dir);
    fs::create_dir_all(&b.config).expect("mkdir");
    let path = b.config.join("graphics.json");
    fs::write(&path, "{\n  \"Graphics\": {\n    \"Width\": 1280,\n    \"Height\": 720\n  },\n  \"Other\": 1\n}\n").expect("seed");

    let plan = plan_of(vec![file_edit(
        Base::Config,
        "graphics.json",
        ConfigFormat::Json,
        vec![set("Graphics.Width", ConfigValue::Int(2560)), set("Graphics.Height", ConfigValue::Int(1440))],
    )]);
    assert_eq!(plan.apply_config(&b).expect("applied"), std::slice::from_ref(&path));

    assert_eq!(
        fs::read_to_string(&path).expect("read"),
        "{\n  \"Graphics\": {\n    \"Width\": 2560,\n    \"Height\": 1440\n  },\n  \"Other\": 1\n}\n"
    );
    assert_eq!(
        fs::read_to_string(b.config.join("graphics.json.reclaw-orig")).expect("backup"),
        "{\n  \"Graphics\": {\n    \"Width\": 1280,\n    \"Height\": 720\n  },\n  \"Other\": 1\n}\n"
    );
}

#[test]
fn apply_config_creates_a_missing_file_and_its_directories() {
    let dir = TempDir::new();
    let b = bases(&dir);
    let plan = plan_of(vec![file_edit(Base::Data, "user/video.cfg", ConfigFormat::KeyValue, vec![set("fps", ConfigValue::Int(144))])]);
    let written = plan.apply_config(&b).expect("applied");
    assert_eq!(written, [b.data.join("user/video.cfg")]);
    assert_eq!(fs::read_to_string(&written[0]).expect("read"), "fps 144\n");
    assert!(!b.data.join("user/video.cfg.reclaw-orig").exists(), "there was no original to keep");
}

#[test]
fn apply_config_handles_several_files_in_order() {
    let dir = TempDir::new();
    let b = bases(&dir);
    let plan = plan_of(vec![
        file_edit(Base::Install, "game.ini", ConfigFormat::Ini, vec![set("Video.Width", ConfigValue::Int(1920))]),
        file_edit(Base::Config, "settings.toml", ConfigFormat::Toml, vec![set("video.vsync", ConfigValue::Bool(false))]),
    ]);
    let written = plan.apply_config(&b).expect("applied");
    assert_eq!(written, [b.install.join("game.ini"), b.config.join("settings.toml")]);
    assert_eq!(fs::read_to_string(&written[0]).expect("read"), "[Video]\nWidth = 1920\n");
    assert_eq!(fs::read_to_string(&written[1]).expect("read"), "[video]\nvsync = false\n");
}

#[test]
fn applying_twice_gives_the_same_files() {
    let dir = TempDir::new();
    let b = bases(&dir);
    let plan = plan_of(vec![file_edit(Base::Config, "g.json", ConfigFormat::Json, vec![set("a.b", ConfigValue::Int(1))])]);
    plan.apply_config(&b).expect("first");
    let first = fs::read_to_string(b.config.join("g.json")).expect("read");
    plan.apply_config(&b).expect("second");
    assert_eq!(fs::read_to_string(b.config.join("g.json")).expect("read"), first);
}

#[test]
fn a_file_that_does_not_parse_is_left_untouched_and_named_in_the_error() {
    let dir = TempDir::new();
    let b = bases(&dir);
    fs::create_dir_all(&b.config).expect("mkdir");
    let path = b.config.join("broken.json");
    fs::write(&path, "{ // the game's own comment\n \"a\": 1 }").expect("seed");

    let plan = plan_of(vec![file_edit(Base::Config, "broken.json", ConfigFormat::Json, vec![set("a", ConfigValue::Int(2))])]);
    let err = plan.apply_config(&b).expect_err("comments are not JSON");
    assert!(matches!(err, ConfigEditError::Parse { .. }), "{err}");
    assert!(err.to_string().contains("broken.json"), "the message names the file: {err}");
    assert_eq!(fs::read_to_string(&path).expect("read"), "{ // the game's own comment\n \"a\": 1 }");
    assert!(!b.config.join("broken.json.reclaw-orig").exists());
}

#[test]
fn a_bad_file_stops_the_run_before_any_file_is_written() {
    let dir = TempDir::new();
    let b = bases(&dir);
    fs::create_dir_all(&b.config).expect("mkdir");
    fs::write(b.config.join("broken.toml"), "= nope").expect("seed");

    let plan = plan_of(vec![
        file_edit(Base::Config, "fine.json", ConfigFormat::Json, vec![set("a", ConfigValue::Int(1))]),
        file_edit(Base::Config, "broken.toml", ConfigFormat::Toml, vec![set("a", ConfigValue::Int(1))]),
        file_edit(Base::Config, "later.ini", ConfigFormat::Ini, vec![set("a.b", ConfigValue::Int(1))]),
    ]);
    assert!(plan.apply_config(&b).is_err());
    assert!(!b.config.join("fine.json").exists() && !b.config.join("later.ini").exists());
    assert_eq!(fs::read_to_string(b.config.join("broken.toml")).expect("read"), "= nope");
}

#[test]
fn a_path_that_escapes_writes_nothing_anywhere() {
    let dir = TempDir::new();
    let b = bases(&dir);
    let plan = plan_of(vec![
        file_edit(Base::Config, "fine.json", ConfigFormat::Json, vec![set("a", ConfigValue::Int(1))]),
        file_edit(Base::Config, "../../outside.json", ConfigFormat::Json, vec![set("a", ConfigValue::Int(1))]),
    ]);
    assert!(matches!(plan.apply_config(&b), Err(ConfigEditError::EscapesBase(_))));
    assert!(!b.config.exists(), "{:?}", fs::read_dir(dir.path()).map(|d| d.count()));
    assert!(!dir.path().join("outside.json").exists());
}

#[test]
fn a_bad_edit_path_leaves_the_file_untouched() {
    let dir = TempDir::new();
    let b = bases(&dir);
    fs::create_dir_all(&b.config).expect("mkdir");
    fs::write(b.config.join("g.json"), "{\"a\": 5}").expect("seed");
    let plan = plan_of(vec![file_edit(Base::Config, "g.json", ConfigFormat::Json, vec![set("a.b", ConfigValue::Int(1))])]);
    assert!(matches!(plan.apply_config(&b), Err(ConfigEditError::BadPath { .. })));
    assert_eq!(fs::read_to_string(b.config.join("g.json")).expect("read"), "{\"a\": 5}");
}

#[test]
fn a_file_that_is_not_text_is_an_io_error_and_untouched() {
    let dir = TempDir::new();
    let b = bases(&dir);
    fs::create_dir_all(&b.config).expect("mkdir");
    let path = b.config.join("binary.dat");
    fs::write(&path, [0xff, 0xfe, 0x00, 0x80]).expect("seed");
    let plan = plan_of(vec![file_edit(Base::Config, "binary.dat", ConfigFormat::KeyValue, vec![set("a", ConfigValue::Int(1))])]);
    assert!(matches!(plan.apply_config(&b), Err(ConfigEditError::Io { .. })));
    assert_eq!(fs::read(&path).expect("read"), [0xff, 0xfe, 0x00, 0x80]);
}
