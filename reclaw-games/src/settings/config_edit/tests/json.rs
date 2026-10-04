use super::*;
use serde_json::Value;

fn json(input: &str, edits: &[KeyEdit]) -> String {
    run(ConfigFormat::Json, input, edits)
}

#[test]
fn replaces_a_value_and_keeps_the_key_where_it_was() {
    let input = "{\n  \"zebra\": 1,\n  \"Graphics\": {\n    \"Width\": 1280,\n    \"Height\": 720\n  },\n  \"apple\": 2\n}\n";
    let out = json(input, &[int("Graphics.Width", 1920)]);
    assert_eq!(out, "{\n  \"zebra\": 1,\n  \"Graphics\": {\n    \"Width\": 1920,\n    \"Height\": 720\n  },\n  \"apple\": 2\n}\n");
}

#[test]
fn a_new_key_goes_last_in_its_object() {
    let out = json("{\n  \"b\": 1,\n  \"a\": 2\n}", &[int("c", 3)]);
    assert_eq!(out, "{\n  \"b\": 1,\n  \"a\": 2,\n  \"c\": 3\n}");
}

#[test]
fn missing_objects_are_created() {
    let out = json("{\n  \"keep\": true\n}\n", &[int("Graphics.Window.Width", 1920), int("Graphics.Window.Height", 1080)]);
    let value: Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(value["Graphics"]["Window"]["Width"], 1920);
    assert_eq!(value["Graphics"]["Window"]["Height"], 1080);
    assert_eq!(value["keep"], true);
    assert_eq!(
        out,
        "{\n  \"keep\": true,\n  \"Graphics\": {\n    \"Window\": {\n      \"Width\": 1920,\n      \"Height\": 1080\n    }\n  }\n}\n"
    );
}

#[test]
fn indentation_is_detected_from_the_file() {
    let four = json("{\n    \"a\": {\n        \"b\": 1\n    }\n}\n", &[int("a.b", 2)]);
    assert_eq!(four, "{\n    \"a\": {\n        \"b\": 2\n    }\n}\n");
    let tabs = json("{\n\t\"a\": {\n\t\t\"b\": 1\n\t}\n}\n", &[int("a.b", 2)]);
    assert_eq!(tabs, "{\n\t\"a\": {\n\t\t\"b\": 2\n\t}\n}\n");
    let two = json("{\n  \"a\": {\n    \"b\": 1\n  }\n}\n", &[int("a.b", 2)]);
    assert_eq!(two, "{\n  \"a\": {\n    \"b\": 2\n  }\n}\n");
}

#[test]
fn a_file_with_no_indentation_gets_two_spaces() {
    assert_eq!(json("{\"a\":{\"b\":1}}", &[int("a.b", 2)]), "{\n  \"a\": {\n    \"b\": 2\n  }\n}");
    // Only 2, 4 and tabs are recognised: anything else is written with the default.
    assert_eq!(json("{\n   \"a\": 1\n}", &[int("a", 2)]), "{\n  \"a\": 2\n}");
}

#[test]
fn the_trailing_newline_is_kept_or_not() {
    assert!(json("{\"a\":1}\n", &[int("a", 2)]).ends_with("}\n"));
    assert!(!json("{\"a\":1}", &[int("a", 2)]).ends_with('\n'));
}

#[test]
fn crlf_files_stay_crlf() {
    let out = json("{\r\n  \"a\": 1\r\n}\r\n", &[int("b", 2)]);
    assert_eq!(out, "{\r\n  \"a\": 1,\r\n  \"b\": 2\r\n}\r\n");
}

#[test]
fn a_byte_order_mark_is_kept() {
    let out = json("\u{feff}{\"a\":1}", &[int("a", 2)]);
    assert!(out.starts_with("\u{feff}{"));
    assert!(out.contains("\"a\": 2"));
}

#[test]
fn empty_input_is_a_new_object() {
    assert_eq!(json("", &[int("a.b", 1)]), "{\n  \"a\": {\n    \"b\": 1\n  }\n}");
    assert_eq!(json(" \n", &[flag("on", true)]), "{\n  \"on\": true\n}\n");
    assert_eq!(json("", &[]), "{}");
}

#[test]
fn no_edits_still_normalizes_but_never_loses_data() {
    let input = "{\"a\":[1,2,{\"b\":null}],\"c\":\"d\"}";
    let out = json(input, &[]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::from_str::<Value>(input).expect("valid"));
}

#[test]
fn values_have_their_json_types() {
    let out = json("{}", &[flag("b", true), int("i", -4), edit("f", ConfigValue::Float(1.5)), text("s", "x\"y\n"), text("n", "144")]);
    let value: Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(value["b"], Value::Bool(true));
    assert_eq!(value["i"], -4);
    assert_eq!(value["f"], 1.5);
    assert_eq!(value["s"], "x\"y\n");
    assert_eq!(value["n"], "144", "a Text stays a string");
}

#[test]
fn a_float_that_json_cannot_hold_is_an_error() {
    assert!(matches!(fails(ConfigFormat::Json, "{}", &[edit("f", ConfigValue::Float(f64::NAN))]), ConfigEditError::BadPath { .. }));
    assert!(matches!(fails(ConfigFormat::Json, "{}", &[edit("f", ConfigValue::Float(f64::INFINITY))]), ConfigEditError::BadPath { .. }));
}

#[test]
fn a_replaced_value_may_change_type() {
    let out = json("{\"a\": {\"deep\": [1, 2]}}", &[text("a", "gone")]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::json!({"a": "gone"}));
}

#[test]
fn numeric_segments_index_arrays() {
    let input = "{\n  \"modes\": [\n    {\n      \"w\": 1\n    },\n    {\n      \"w\": 2\n    }\n  ]\n}";
    let out = json(input, &[int("modes.1.w", 9), int("modes.0.h", 5)]);
    let value: Value = serde_json::from_str(&out).expect("valid json");
    assert_eq!(value["modes"], serde_json::json!([{"w": 1, "h": 5}, {"w": 9}]));
}

#[test]
fn an_array_element_can_be_replaced_or_appended() {
    let out = json("{\"a\": [10, 20]}", &[int("a.0", 11), int("a.2", 30)]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::json!({"a": [11, 20, 30]}));
    let nested = json("{\"a\": []}", &[int("a.0.x", 1)]);
    assert_eq!(serde_json::from_str::<Value>(&nested).expect("valid"), serde_json::json!({"a": [{"x": 1}]}));
}

#[test]
fn an_array_can_be_the_root() {
    let out = json("[{\"a\": 1}]", &[int("0.a", 2)]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::json!([{"a": 2}]));
}

#[test]
fn a_numeric_segment_in_an_object_is_just_a_key() {
    let out = json("{\"a\": {}}", &[int("a.0", 1)]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::json!({"a": {"0": 1}}));
}

#[test]
fn bad_array_paths_are_errors() {
    for path in ["a.5", "a.x", "a.-1", "a.2.b"] {
        let err = fails(ConfigFormat::Json, "{\"a\": [1]}", &[int(path, 1)]);
        assert!(matches!(err, ConfigEditError::BadPath { .. }), "{path}: {err}");
    }
}

#[test]
fn a_scalar_in_the_way_is_an_error_but_null_is_replaced() {
    for input in ["{\"a\": 5}", "{\"a\": \"s\"}", "{\"a\": true}"] {
        assert!(matches!(fails(ConfigFormat::Json, input, &[int("a.b", 1)]), ConfigEditError::BadPath { .. }), "{input}");
    }
    let out = json("{\"a\": null}", &[int("a.b", 1)]);
    assert_eq!(serde_json::from_str::<Value>(&out).expect("valid"), serde_json::json!({"a": {"b": 1}}));
    assert!(matches!(fails(ConfigFormat::Json, "5", &[int("a", 1)]), ConfigEditError::BadPath { .. }));
}

#[test]
fn empty_names_in_a_path_are_errors() {
    for path in ["", ".", "a..b", ".a", "a."] {
        assert!(matches!(fails(ConfigFormat::Json, "{}", &[int(path, 1)]), ConfigEditError::BadPath { .. }), "{path:?}");
    }
}

#[test]
fn a_file_that_does_not_parse_is_an_error() {
    for input in ["{ // comment\n \"a\": 1 }", "{\"a\": 1,}", "{\"a\": }", "not json", "{\"a\": 1} trailing"] {
        let err = fails(ConfigFormat::Json, input, &[int("a", 2)]);
        assert!(matches!(err, ConfigEditError::Parse { .. }), "{input}: {err}");
    }
}

#[test]
fn the_last_edit_to_a_path_wins() {
    let out = json("{}", &[int("a", 1), int("a", 2)]);
    assert_eq!(out, "{\n  \"a\": 2\n}");
}

#[test]
fn a_failing_edit_returns_no_partial_text() {
    assert!(apply_edits(ConfigFormat::Json, "{}", &[int("a", 1), int("a.b", 2)]).is_err());
}

#[test]
fn applying_the_same_edits_twice_changes_nothing_more() {
    let edits = [int("Graphics.Width", 1920), flag("Graphics.VSync", false), text("Graphics.Upscaler", "fsr1")];
    let once = json("{\n    \"Graphics\": {\"Width\": 1}\n}\n", &edits);
    assert_eq!(json(&once, &edits), once);
}

#[test]
fn text_survives_unicode_and_escapes() {
    let out = json("{\"名前\": \"é\"}", &[text("title", "ü \u{1F3AE}")]);
    assert!(out.contains("\"名前\": \"é\"") && out.contains("ü \u{1F3AE}"), "{out}");
}
