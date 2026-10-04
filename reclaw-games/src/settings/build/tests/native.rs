use super::*;

#[test]
fn native_uses_the_primary_monitor() {
    let p = plan_of(&resolution_args(), &two_monitors(DisplayServer::X11), &[(SettingKey::Resolution, SettingValue::Native)]);
    assert_eq!(p.args, strings(&["--width", "2560", "--height", "1440"]));
    let first_is_not_primary = env(DisplayServer::X11, vec![monitor("A", "A", 1920, 1080, false), monitor("B", "B", 3840, 2160, true)]);
    let p = plan_of(&resolution_args(), &first_is_not_primary, &[(SettingKey::Resolution, SettingValue::Native)]);
    assert_eq!(p.args, strings(&["--width", "3840", "--height", "2160"]));
}

#[test]
fn native_uses_the_monitor_the_user_chose() {
    let mut c = resolution_args();
    c.settings.push(bind(SettingKey::Monitor, Constraint::None, vec![args(None, &["--display", "{value}"])]));
    let p = plan_of(
        &c,
        &two_monitors(DisplayServer::X11),
        &[(SettingKey::Resolution, SettingValue::Native), (SettingKey::Monitor, SettingValue::choice("HDMI-1"))],
    );
    assert_eq!(p.args, strings(&["--width", "3840", "--height", "2160", "--display", "HDMI-1"]));
}

#[test]
fn a_monitor_the_game_does_not_declare_does_not_move_native() {
    let p = plan_of(
        &resolution_args(),
        &two_monitors(DisplayServer::X11),
        &[(SettingKey::Resolution, SettingValue::Native), (SettingKey::Monitor, SettingValue::choice("HDMI-1"))],
    );
    assert_eq!(p.args, strings(&["--width", "2560", "--height", "1440"]));
}

#[test]
fn native_without_any_monitor_emits_nothing() {
    assert!(plan_of(&resolution_args(), &DisplayEnvironment::unknown(), &[(SettingKey::Resolution, SettingValue::Native)]).is_empty());
    // {value} needs no monitor.
    let c = caps(vec![bind(SettingKey::Resolution, Constraint::None, vec![args(None, &["--res", "{value}"])])]);
    assert_eq!(
        plan_of(&c, &DisplayEnvironment::unknown(), &[(SettingKey::Resolution, SettingValue::Native)]).args,
        strings(&["--res", "native"])
    );
}
