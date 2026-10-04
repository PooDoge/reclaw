use super::*;

fn ini(input: &str, edits: &[KeyEdit]) -> String {
    run(ConfigFormat::Ini, input, edits)
}

const SAMPLE: &str = "; engine settings\n[Display]\nWidth=1280\nHeight = 720\n; Fullscreen=0\n\n[Audio]\n# loud\nVolume = 8\n";

#[test]
fn the_spacing_of_each_existing_line_survives() {
    let out = ini(SAMPLE, &[int("Display.Width", 1920), int("Display.Height", 1080)]);
    assert_eq!(out, "; engine settings\n[Display]\nWidth=1920\nHeight = 1080\n; Fullscreen=0\n\n[Audio]\n# loud\nVolume = 8\n");
}

#[test]
fn unrelated_lines_are_byte_for_byte_the_same() {
    let messy = "  ; indented comment\n[Display]  \n  Width   =   1280   \nheight=1\n\n\n[Other]\nodd line without equals\nx = 1\n";
    let out = ini(messy, &[int("Display.height", 2)]);
    unrelated_lines_survive(messy, &out, &["height=1"]);
    assert!(out.contains("height=2\n"));
    let out = ini(messy, &[int("Display.Width", 9)]);
    assert!(out.contains("  Width   =   9\n"), "the indent and the spacing stay: {out:?}");
}

#[test]
fn a_missing_key_is_added_at_the_end_of_its_section() {
    let out = ini(SAMPLE, &[text("Display.Upscaler", "fsr1")]);
    assert_eq!(
        out,
        "; engine settings\n[Display]\nWidth=1280\nHeight = 720\nUpscaler = fsr1\n; Fullscreen=0\n\n[Audio]\n# loud\nVolume = 8\n"
    );
}

#[test]
fn a_new_key_copies_the_spacing_of_its_section() {
    let tight = ini("[a]\nx=1\n\n[b]\ny = 2\n", &[int("a.z", 3), int("b.w", 4)]);
    assert_eq!(tight, "[a]\nx=1\nz=3\n\n[b]\ny = 2\nw = 4\n");
}

#[test]
fn a_new_key_in_a_section_without_keys_goes_under_the_header() {
    assert_eq!(ini("[a]\n\n[b]\nx = 1\n", &[int("a.k", 1)]), "[a]\nk = 1\n\n[b]\nx = 1\n");
    // The file's own style is used when the section has none to copy.
    assert_eq!(ini("[a]\n[b]\nx=1\n", &[int("a.k", 1)]), "[a]\nk=1\n[b]\nx=1\n");
}

#[test]
fn a_missing_section_goes_at_the_end_of_the_file() {
    let out = ini(SAMPLE, &[int("Video.Fps", 60)]);
    assert!(out.ends_with("Volume = 8\n\n[Video]\nFps=60\n"), "{out:?}");
    assert_eq!(ini("a=1\n\n", &[int("S.k", 1)]), "a=1\n\n[S]\nk=1\n");
}

#[test]
fn a_missing_section_in_an_empty_file() {
    assert_eq!(ini("", &[int("Display.Width", 1920), int("Display.Height", 1080)]), "[Display]\nWidth = 1920\nHeight = 1080\n");
}

#[test]
fn the_path_splits_at_the_last_dot() {
    let out = ini("[Render.Settings]\nx = 1\n", &[int("Render.Settings.x", 2), int("Render.Settings.y", 3)]);
    assert_eq!(out, "[Render.Settings]\nx = 2\ny = 3\n");
    assert_eq!(ini("", &[int("A.B.c", 1)]), "[A.B]\nc = 1\n");
}

#[test]
fn no_dot_means_before_any_section() {
    let with_globals = ini("fps=30\n[a]\nx=1\n", &[int("fps", 60), int("vsync", 0)]);
    assert_eq!(with_globals, "fps=60\nvsync=0\n[a]\nx=1\n");
    // Without global keys it goes above the first header, after the leading comments.
    assert_eq!(ini("; hi\n\n[a]\nx = 1\n", &[int("fps", 60)]), "; hi\nfps = 60\n\n[a]\nx = 1\n");
    assert_eq!(ini("[a]\nx = 1\n", &[int("fps", 60)]), "fps = 60\n\n[a]\nx = 1\n");
    assert_eq!(ini("; only comments\n", &[int("fps", 60)]), "; only comments\nfps = 60\n");
}

#[test]
fn a_global_key_is_not_confused_with_a_sectioned_one() {
    let out = ini("x = 1\n[a]\nx = 2\n", &[int("x", 9)]);
    assert_eq!(out, "x = 9\n[a]\nx = 2\n");
    let out = ini("x = 1\n[a]\nx = 2\n", &[int("a.x", 9)]);
    assert_eq!(out, "x = 1\n[a]\nx = 9\n");
}

#[test]
fn commented_out_keys_are_not_matched() {
    // Nothing in the section is a key, so the new one goes under the header, above the comments.
    let out = ini("[a]\n;Width=1\n#Width=2\n", &[int("a.Width", 3)]);
    assert_eq!(out, "[a]\nWidth = 3\n;Width=1\n#Width=2\n");
}

#[test]
fn names_are_case_sensitive() {
    let out = ini("[Display]\nwidth=1\n", &[int("Display.Width", 2), int("display.width", 3)]);
    assert_eq!(out, "[Display]\nwidth=1\nWidth=2\n\n[display]\nwidth=3\n");
}

#[test]
fn every_copy_of_a_key_is_updated() {
    let out = ini("[a]\nx=1\ny=2\nx=3\n[b]\nx=4\n[a]\nx=5\n", &[int("a.x", 9)]);
    assert_eq!(out, "[a]\nx=9\ny=2\nx=9\n[b]\nx=4\n[a]\nx=9\n");
}

#[test]
fn a_repeated_section_gets_the_new_key_in_its_last_block() {
    assert_eq!(ini("[a]\nx=1\n[b]\ny=1\n[a]\nz=1\n", &[int("a.new", 2)]), "[a]\nx=1\n[b]\ny=1\n[a]\nz=1\nnew=2\n");
}

#[test]
fn section_headers_are_matched_loosely() {
    assert_eq!(ini("[ Display ]  ; main\nx = 1\n", &[int("Display.x", 2)]), "[ Display ]  ; main\nx = 2\n");
}

#[test]
fn an_inline_value_with_equals_signs_is_replaced_whole() {
    assert_eq!(ini("[a]\nargs = -a=1 -b=2\n", &[text("a.args", "-c=3")]), "[a]\nargs = -c=3\n");
}

#[test]
fn line_endings_and_the_final_newline_are_kept() {
    assert_eq!(ini("[a]\r\nx=1\r\n", &[int("a.x", 2), int("a.y", 3), int("b.z", 4)]), "[a]\r\nx=2\r\ny=3\r\n\r\n[b]\r\nz=4\r\n");
    assert_eq!(ini("[a]\nx=1", &[int("a.x", 2)]), "[a]\nx=2");
    assert_eq!(ini("[a]\nx=1", &[int("a.y", 2)]), "[a]\nx=1\ny=2");
    assert_eq!(ini("[a]\nx=1", &[int("b.y", 2)]), "[a]\nx=1\n\n[b]\ny=2");
}

#[test]
fn values_are_written_as_text() {
    let out = ini("", &[flag("a.b", true), int("a.i", -3), edit("a.f", ConfigValue::Float(2.0)), text("a.s", "two words")]);
    assert_eq!(out, "[a]\nb = true\ni = -3\nf = 2.0\ns = two words\n");
}

#[test]
fn a_value_with_a_line_break_is_an_error() {
    assert!(matches!(fails(ConfigFormat::Ini, "", &[text("a.b", "x\ny")]), ConfigEditError::BadPath { .. }));
    assert!(matches!(fails(ConfigFormat::Ini, "", &[text("a.b", "x\ry")]), ConfigEditError::BadPath { .. }));
}

#[test]
fn unusable_names_are_errors() {
    for path in ["", ".", "a.", ".k", "a.k=v", "a.;k", "a.#k", "a.[k", "a]b.k", " a.k", "a. k", "a.k "] {
        assert!(matches!(fails(ConfigFormat::Ini, "", &[int(path, 1)]), ConfigEditError::BadPath { .. }), "{path:?}");
    }
}

#[test]
fn applying_the_same_edits_twice_changes_nothing_more() {
    let edits = [int("Display.Width", 1920), int("Video.Fps", 60), int("global", 1), flag("Display.VSync", false)];
    let once = ini(SAMPLE, &edits);
    assert_eq!(ini(&once, &edits), once);
}

#[test]
fn nothing_to_edit_returns_the_text_untouched() {
    for input in [SAMPLE, "", "no newline", "\r\n[a]\r\n", "weird\n\n\n"] {
        assert_eq!(ini(input, &[]), input);
    }
}
