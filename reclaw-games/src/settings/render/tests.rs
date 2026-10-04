use super::*;

fn ctx<'a>(value: &'a SettingValue, size: Option<Size>, map: &'a BTreeMap<String, String>) -> Context<'a> {
    Context { value, size, map }
}

fn fill(template: &str, value: &SettingValue) -> Option<String> {
    let size = match value {
        SettingValue::Size(s) => Some(*s),
        _ => None,
    };
    render(template, &ctx(value, size, &BTreeMap::new()))
}

#[test]
fn value_text_per_shape() {
    assert_eq!(value_text(&SettingValue::Bool(true)), "true");
    assert_eq!(value_text(&SettingValue::Bool(false)), "false");
    assert_eq!(value_text(&SettingValue::Int(-3)), "-3");
    assert_eq!(value_text(&SettingValue::Int(144)), "144");
    assert_eq!(value_text(&SettingValue::choice("aniso16")), "aniso16");
    assert_eq!(value_text(&SettingValue::Size(Size::new(2560, 1440))), "2560x1440");
    assert_eq!(value_text(&SettingValue::Native), "native");
}

#[test]
fn value_placeholder() {
    assert_eq!(fill("--fps={value}", &SettingValue::Int(60)).as_deref(), Some("--fps=60"));
    assert_eq!(fill("{value}", &SettingValue::Bool(false)).as_deref(), Some("false"));
    assert_eq!(fill("{value}{value}", &SettingValue::choice("4x")).as_deref(), Some("4x4x"));
    assert_eq!(fill("--res={value}", &SettingValue::Size(Size::new(1920, 1080))).as_deref(), Some("--res=1920x1080"));
    assert_eq!(fill("{value}", &SettingValue::Native).as_deref(), Some("native"));
}

#[test]
fn width_and_height_come_from_the_size() {
    let value = SettingValue::Size(Size::new(2560, 1440));
    assert_eq!(fill("{width}x{height}", &value).as_deref(), Some("2560x1440"));
    assert_eq!(fill("{height}", &value).as_deref(), Some("1440"));
}

#[test]
fn width_without_a_size_means_no_output() {
    assert_eq!(fill("{width}", &SettingValue::Int(5)), None);
    assert_eq!(fill("--h={height}", &SettingValue::choice("x")), None);
    // Native with no monitor to ask has no size either.
    assert_eq!(render("{width}", &ctx(&SettingValue::Native, None, &BTreeMap::new())), None);
    let no_map = BTreeMap::new();
    let native = ctx(&SettingValue::Native, Some(Size::new(3440, 1440)), &no_map);
    assert_eq!(render("{width}x{height} {value}", &native).as_deref(), Some("3440x1440 native"));
}

#[test]
fn mapped_looks_the_value_text_up() {
    let map: BTreeMap<String, String> = [("true", "On"), ("4x", "MSAA_4"), ("144", "high"), ("1920x1080", "FHD"), ("native", "Auto")]
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .into();
    let mapped = |value: SettingValue| render("{mapped}", &ctx(&value, None, &map));
    assert_eq!(mapped(SettingValue::Bool(true)).as_deref(), Some("On"));
    assert_eq!(mapped(SettingValue::choice("4x")).as_deref(), Some("MSAA_4"));
    assert_eq!(mapped(SettingValue::Int(144)).as_deref(), Some("high"));
    assert_eq!(mapped(SettingValue::Size(Size::new(1920, 1080))).as_deref(), Some("FHD"));
    assert_eq!(mapped(SettingValue::Native).as_deref(), Some("Auto"));
    assert_eq!(mapped(SettingValue::Bool(false)), None, "no entry, no output");
    assert_eq!(mapped(SettingValue::choice("8x")), None);
}

#[test]
fn only_the_four_placeholders_are_replaced() {
    let value = SettingValue::Int(1);
    assert_eq!(fill("{other} {VALUE} { value } {value", &value).as_deref(), Some("{other} {VALUE} { value } {value"));
    assert_eq!(fill("}{", &value).as_deref(), Some("}{"));
    assert_eq!(fill("{}", &value).as_deref(), Some("{}"));
    assert_eq!(fill("{{value}}", &value).as_deref(), Some("{1}"), "there is no escape: the inner one is replaced");
    assert_eq!(fill("", &value).as_deref(), Some(""));
    assert_eq!(fill("plain", &value).as_deref(), Some("plain"));
}

#[test]
fn replaced_text_is_not_scanned_again() {
    assert_eq!(fill("{value}", &SettingValue::choice("{width}")).as_deref(), Some("{width}"));
    assert_eq!(fill("{value}-{value}", &SettingValue::choice("{value}")).as_deref(), Some("{value}-{value}"));
}

#[test]
fn text_around_placeholders_may_be_any_unicode() {
    assert_eq!(fill("é{value}ü{", &SettingValue::Int(2)).as_deref(), Some("é2ü{"));
    assert_eq!(fill("日本{value}語", &SettingValue::Int(2)).as_deref(), Some("日本2語"));
}

#[test]
fn auto_types_booleans_integers_and_text() {
    let auto = |s| typed(s, ValueType::Auto);
    assert_eq!(auto("true"), Some(ConfigValue::Bool(true)));
    assert_eq!(auto("false"), Some(ConfigValue::Bool(false)));
    assert_eq!(auto("144"), Some(ConfigValue::Int(144)));
    assert_eq!(auto("-5"), Some(ConfigValue::Int(-5)));
    assert_eq!(auto("0"), Some(ConfigValue::Int(0)));
    assert_eq!(auto("fsr1"), Some(ConfigValue::Text("fsr1".into())));
    assert_eq!(auto("2560x1440"), Some(ConfigValue::Text("2560x1440".into())));
    assert_eq!(auto("1.5"), Some(ConfigValue::Text("1.5".into())), "floats are not guessed");
    assert_eq!(auto("True"), Some(ConfigValue::Text("True".into())));
    assert_eq!(auto(""), Some(ConfigValue::Text(String::new())));
}

#[test]
fn auto_keeps_numbers_that_are_really_names() {
    assert_eq!(typed("007", ValueType::Auto), Some(ConfigValue::Text("007".into())));
    assert_eq!(typed("+5", ValueType::Auto), Some(ConfigValue::Text("+5".into())));
    assert_eq!(typed("-0", ValueType::Auto), Some(ConfigValue::Text("-0".into())));
    assert_eq!(typed(" 5", ValueType::Auto), Some(ConfigValue::Text(" 5".into())));
    assert_eq!(typed("99999999999999999999", ValueType::Auto), Some(ConfigValue::Text("99999999999999999999".into())));
}

#[test]
fn string_is_always_text() {
    assert_eq!(typed("144", ValueType::String), Some(ConfigValue::Text("144".into())));
    assert_eq!(typed("true", ValueType::String), Some(ConfigValue::Text("true".into())));
}

#[test]
fn int_bool_and_float_refuse_text_that_is_not_theirs() {
    assert_eq!(typed("144", ValueType::Int), Some(ConfigValue::Int(144)));
    assert_eq!(typed("fsr1", ValueType::Int), None);
    assert_eq!(typed("1.5", ValueType::Int), None);
    assert_eq!(typed("true", ValueType::Bool), Some(ConfigValue::Bool(true)));
    assert_eq!(typed("false", ValueType::Bool), Some(ConfigValue::Bool(false)));
    assert_eq!(typed("1", ValueType::Bool), None);
    assert_eq!(typed("1.5", ValueType::Float), Some(ConfigValue::Float(1.5)));
    assert_eq!(typed("2", ValueType::Float), Some(ConfigValue::Float(2.0)));
    assert_eq!(typed("abc", ValueType::Float), None);
    assert_eq!(typed("NaN", ValueType::Float), None);
    assert_eq!(typed("inf", ValueType::Float), None);
}
