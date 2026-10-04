use super::*;

fn kv(input: &str, edits: &[KeyEdit]) -> String {
    run(ConfigFormat::KeyValue, input, edits)
}

#[test]
fn the_separator_of_each_line_is_kept() {
    let input = "width 1280\nheight=720\nfps = 60\nvsync   1\n";
    let out = kv(input, &[int("width", 1920), int("height", 1080), int("fps", 144), flag("vsync", false)]);
    assert_eq!(out, "width 1920\nheight=1080\nfps = 144\nvsync   false\n");
}

#[test]
fn unrelated_lines_are_byte_for_byte_the_same() {
    let input = "# video\n  width   1280  \n\n; audio\nvolume=3\n// note\nrest\tof\tline\n";
    let out = kv(input, &[int("volume", 9)]);
    unrelated_lines_survive(input, &out, &["volume=3"]);
    assert!(out.contains("volume=9\n"));
    assert!(kv(input, &[int("width", 1)]).contains("  width   1\n"), "indent and separator stay");
}

#[test]
fn a_new_key_copies_the_first_key_lines_separator() {
    assert_eq!(kv("a = 1\nb=2\n", &[int("c", 3)]), "a = 1\nb=2\nc = 3\n");
    assert_eq!(kv("a=1\nb = 2\n", &[int("c", 3)]), "a=1\nb = 2\nc=3\n");
    assert_eq!(kv("a\t1\n", &[int("c", 3)]), "a\t1\nc\t3\n");
    assert_eq!(kv("a    1\n", &[int("c", 3)]), "a    1\nc    3\n");
}

#[test]
fn comments_and_blank_lines_do_not_provide_the_separator() {
    assert_eq!(kv("# a = b\n\nx=1\n", &[int("y", 2)]), "# a = b\n\nx=1\ny=2\n");
}

#[test]
fn a_single_space_is_the_default() {
    assert_eq!(kv("", &[int("width", 1920), int("height", 1080)]), "width 1920\nheight 1080\n");
    assert_eq!(kv("# only a comment\n", &[int("width", 1920)]), "# only a comment\nwidth 1920\n");
    assert_eq!(kv("lonely\n", &[int("width", 1920)]), "lonely\nwidth 1920\n");
}

#[test]
fn a_new_key_goes_after_the_last_real_line() {
    assert_eq!(kv("a 1\n\n\n", &[int("b", 2)]), "a 1\nb 2\n\n\n");
    assert_eq!(kv("a 1\n# end\n", &[int("b", 2)]), "a 1\n# end\nb 2\n");
}

#[test]
fn the_whole_path_is_the_key() {
    let out = kv("Graphics.Width 1280\nGraphics 5\n", &[int("Graphics.Width", 1920), int("Audio.Volume", 3)]);
    assert_eq!(out, "Graphics.Width 1920\nGraphics 5\nAudio.Volume 3\n");
}

#[test]
fn comment_lines_are_never_matched() {
    for comment in ["#width 1", ";width 1", "//width 1"] {
        let out = kv(&format!("{comment}\n"), &[int("width", 2)]);
        assert_eq!(out, format!("{comment}\nwidth 2\n"), "{comment}");
    }
}

#[test]
fn every_copy_of_a_key_is_updated() {
    assert_eq!(kv("x 1\ny 2\nx 3\n", &[int("x", 9)]), "x 9\ny 2\nx 9\n");
}

#[test]
fn a_key_must_match_whole() {
    assert_eq!(kv("width_max 5\nwidth 1\n", &[int("width", 2)]), "width_max 5\nwidth 2\n");
    assert_eq!(kv("widths 5\n", &[int("width", 2)]), "widths 5\nwidth 2\n");
}

#[test]
fn a_lone_key_gets_a_value() {
    assert_eq!(kv("a 1\nflag\n", &[int("flag", 1)]), "a 1\nflag 1\n");
    assert_eq!(kv("a=1\nflag\n", &[int("flag", 1)]), "a=1\nflag=1\n");
}

#[test]
fn the_rest_of_the_line_is_the_value() {
    assert_eq!(kv("title Star Fall 64 // old\n", &[text("title", "New")]), "title New\n");
    assert_eq!(kv("path=a=b\n", &[text("path", "c")]), "path=c\n");
}

#[test]
fn line_endings_and_the_final_newline_are_kept() {
    assert_eq!(kv("a 1\r\nb 2\r\n", &[int("a", 9), int("c", 3)]), "a 9\r\nb 2\r\nc 3\r\n");
    assert_eq!(kv("a 1", &[int("a", 2)]), "a 2");
    assert_eq!(kv("a 1", &[int("b", 2)]), "a 1\nb 2");
}

#[test]
fn values_are_written_as_text() {
    let out = kv("", &[flag("a", true), int("b", -3), edit("c", ConfigValue::Float(0.5)), text("d", "two words")]);
    assert_eq!(out, "a true\nb -3\nc 0.5\nd two words\n");
}

#[test]
fn bad_keys_and_values_are_errors() {
    for path in ["", "two words", "a=b", "#c", ";c", "//c", "tab\tkey", "nl\nkey"] {
        assert!(matches!(fails(ConfigFormat::KeyValue, "", &[int(path, 1)]), ConfigEditError::BadPath { .. }), "{path:?}");
    }
    assert!(matches!(fails(ConfigFormat::KeyValue, "", &[text("a", "x\ny")]), ConfigEditError::BadPath { .. }));
}

#[test]
fn applying_the_same_edits_twice_changes_nothing_more() {
    let edits = [int("width", 1920), flag("vsync", false), int("new.key", 3)];
    let once = kv("width 1\n# c\nvsync=1\n", &edits);
    assert_eq!(kv(&once, &edits), once);
}

#[test]
fn nothing_to_edit_returns_the_text_untouched() {
    for input in ["", "a 1\nb=2\n", "no newline", "\r\n", "x\n\n\n", "= odd\n"] {
        assert_eq!(kv(input, &[]), input);
    }
}
