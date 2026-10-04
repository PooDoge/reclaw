use super::*;

fn toml(input: &str, edits: &[KeyEdit]) -> String {
    run(ConfigFormat::Toml, input, edits)
}

const SAMPLE: &str = "# Game settings\ntitle = \"Starfall\"   # shown in the launcher\n\n[graphics]\n# pixels\nwidth = 1280  # was 720p\nheight = 720\nvsync = true\n\n[audio]\nvolume = 0.8\n";

#[test]
fn comments_and_layout_survive_an_edit() {
    let out = toml(SAMPLE, &[int("graphics.width", 1920)]);
    assert_eq!(out, SAMPLE.replace("width = 1280  # was 720p", "width = 1920  # was 720p"));
}

#[test]
fn unrelated_lines_are_byte_for_byte_the_same() {
    let out = toml(SAMPLE, &[flag("graphics.vsync", false), int("graphics.height", 1080)]);
    unrelated_lines_survive(SAMPLE, &out, &["height = 720", "vsync = true"]);
    assert!(out.contains("height = 1080") && out.contains("vsync = false"));
}

#[test]
fn a_new_key_joins_its_table() {
    let out = toml(SAMPLE, &[text("graphics.upscaler", "fsr1")]);
    assert!(out.contains("vsync = true\nupscaler = \"fsr1\"\n"), "{out}");
    assert!(out.contains("[audio]"));
}

#[test]
fn a_new_top_level_key() {
    let out = toml(SAMPLE, &[int("fps", 144)]);
    assert!(out.starts_with("# Game settings\ntitle = \"Starfall\"   # shown in the launcher\nfps = 144\n"), "{out}");
}

#[test]
fn missing_tables_are_created_without_empty_parents() {
    let out = toml("a = 1\n", &[int("video.window.width", 1920), int("video.window.height", 1080)]);
    assert_eq!(out, "a = 1\n\n[video.window]\nwidth = 1920\nheight = 1080\n");
    let doc: toml_edit::DocumentMut = out.parse().expect("valid toml");
    assert_eq!(doc["video"]["window"]["width"].as_integer(), Some(1920));
}

#[test]
fn an_empty_file_is_a_new_document() {
    assert_eq!(toml("", &[int("width", 1920)]), "width = 1920\n");
    assert_eq!(toml("", &[int("a.b", 1), text("c", "d")]), "c = \"d\"\n\n[a]\nb = 1\n");
}

#[test]
fn values_have_their_toml_types() {
    let out = toml(
        "",
        &[
            flag("b", true),
            int("i", -4),
            edit("f", ConfigValue::Float(1.5)),
            edit("g", ConfigValue::Float(2.0)),
            text("s", "quote\" and \\ slash"),
            text("n", "144"),
        ],
    );
    let doc: toml_edit::DocumentMut = out.parse().expect("valid toml");
    assert_eq!(doc["b"].as_bool(), Some(true));
    assert_eq!(doc["i"].as_integer(), Some(-4));
    assert_eq!(doc["f"].as_float(), Some(1.5));
    assert_eq!(doc["g"].as_float(), Some(2.0), "a whole float stays a float");
    assert_eq!(doc["s"].as_str(), Some("quote\" and \\ slash"));
    assert_eq!(doc["n"].as_str(), Some("144"));
}

#[test]
fn dotted_keys_and_inline_tables_are_reached() {
    let out = toml(
        "window.width = 1\nvideo = { width = 2, height = 3 }\n",
        &[int("window.width", 10), int("video.width", 20), int("video.fps", 60)],
    );
    let doc: toml_edit::DocumentMut = out.parse().expect("valid toml");
    assert_eq!(doc["window"]["width"].as_integer(), Some(10));
    assert_eq!(doc["video"]["width"].as_integer(), Some(20));
    assert_eq!(doc["video"]["height"].as_integer(), Some(3));
    assert_eq!(doc["video"]["fps"].as_integer(), Some(60));
}

#[test]
fn table_order_is_kept() {
    let out = toml("[z]\na = 1\n\n[a]\nb = 2\n", &[int("a.b", 3), int("z.a", 4)]);
    assert_eq!(out, "[z]\na = 4\n\n[a]\nb = 3\n");
}

#[test]
fn arrays_cannot_be_indexed() {
    let err = fails(ConfigFormat::Toml, "[[mode]]\nw = 1\n", &[int("mode.0.w", 2)]);
    assert!(matches!(err, ConfigEditError::BadPath { .. }), "{err}");
    let err = fails(ConfigFormat::Toml, "list = [1, 2]\n", &[int("list.0", 2)]);
    assert!(matches!(err, ConfigEditError::BadPath { .. }), "{err}");
}

#[test]
fn a_value_in_the_way_or_a_table_to_replace_is_an_error() {
    assert!(matches!(fails(ConfigFormat::Toml, "a = 1\n", &[int("a.b", 2)]), ConfigEditError::BadPath { .. }));
    assert!(matches!(fails(ConfigFormat::Toml, "[a]\nb = 1\n", &[int("a", 2)]), ConfigEditError::BadPath { .. }));
}

#[test]
fn empty_names_in_a_path_are_errors() {
    for path in ["", ".a", "a.", "a..b"] {
        assert!(matches!(fails(ConfigFormat::Toml, "", &[int(path, 1)]), ConfigEditError::BadPath { .. }), "{path:?}");
    }
}

#[test]
fn a_file_that_does_not_parse_is_an_error() {
    for input in ["a = ", "[unclosed", "a = 1\na = 2\n", "= 3"] {
        assert!(matches!(fails(ConfigFormat::Toml, input, &[int("b", 1)]), ConfigEditError::Parse { .. }), "{input:?}");
    }
}

#[test]
fn crlf_files_stay_crlf() {
    let out = toml("# top\r\n[a]\r\nb = 1\r\n", &[int("a.b", 2), int("a.c", 3), int("d.e", 4)]);
    assert_eq!(out, "# top\r\n[a]\r\nb = 2\r\nc = 3\r\n\r\n[d]\r\ne = 4\r\n");
}

#[test]
fn applying_the_same_edits_twice_changes_nothing_more() {
    let edits = [int("graphics.width", 1920), flag("graphics.vsync", false), text("video.upscaler", "fsr1")];
    let once = toml(SAMPLE, &edits);
    assert_eq!(toml(&once, &edits), once);
}
