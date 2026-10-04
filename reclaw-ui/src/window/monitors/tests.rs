use super::*;

fn mon(name: Option<&str>, size: (u32, u32), position: (i32, i32)) -> RawMonitor {
    RawMonitor { name: name.map(str::to_string), size, position, scale: 1., refresh_mhz: Some(60_000), primary: false }
}

fn vars<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |k| pairs.iter().find(|(key, _)| *key == k).map(|(_, v)| v.to_string())
}

#[test]
fn monitors_are_numbered_left_to_right_then_top_to_bottom() {
    let raw = vec![
        mon(Some("HDMI-1"), (1920, 1080), (2560, 0)),
        mon(Some("DP-1"), (2560, 1440), (0, 0)),
        mon(Some("DP-2"), (1920, 1080), (0, 1440)),
    ];
    let names: Vec<_> = arrange(raw).into_iter().map(|m| m.name.unwrap()).collect();
    assert_eq!(names, ["DP-1", "DP-2", "HDMI-1"]);
    assert_eq!(monitor_number(0), "Monitor 1");
    assert_eq!(monitor_number(2), "Monitor 3");
}

#[test]
fn the_environment_carries_size_refresh_and_a_unique_id_for_each() {
    let mut a = mon(Some("DP-1"), (2560, 1440), (0, 0));
    a.refresh_mhz = Some(144_000);
    let env = environment(DisplayServer::Wayland, &[mon(Some("DP-1"), (1920, 1080), (2560, 0)), a]);
    assert_eq!(env.server, DisplayServer::Wayland);
    assert_eq!(env.monitors.len(), 2);
    let first = &env.monitors[0];
    assert_eq!((first.id.as_str(), first.native, first.refresh_mhz), ("DP-1", Size::new(2560, 1440), 144_000));
    assert_eq!(env.monitors[1].id, "DP-1-2", "two monitors with one name still have distinct ids");
}

#[test]
fn a_monitor_without_a_name_is_called_by_its_number() {
    let env = environment(DisplayServer::X11, &[mon(None, (1920, 1080), (0, 0)), mon(Some("  "), (1280, 720), (1920, 0))]);
    assert_eq!(env.monitors[0].name, "Monitor 1");
    assert_eq!(env.monitors[0].id, "monitor-1");
    assert_eq!(env.monitors[1].name, "Monitor 2", "blank is as good as missing");
    assert_eq!(env.monitors[1].refresh_mhz, 60_000);
}

#[test]
fn the_primary_is_the_systems_answer_then_the_origin_then_the_first() {
    let origin = mon(Some("A"), (1920, 1080), (0, 0));
    let right = mon(Some("B"), (1920, 1080), (1920, 0));
    let primary_of = |monitors: &[RawMonitor]| environment(DisplayServer::X11, monitors).primary().map(|m| m.id.clone());

    assert_eq!(primary_of(&[origin.clone(), right.clone()]).as_deref(), Some("A"), "the origin, when nothing is marked");
    let mut marked = right.clone();
    marked.primary = true;
    assert_eq!(primary_of(&[origin, marked]).as_deref(), Some("B"), "the system knows best");
    let off_origin = mon(Some("C"), (1920, 1080), (-1920, 100));
    assert_eq!(primary_of(&[off_origin, right]).as_deref(), Some("C"), "no origin monitor: the first in order");
    assert_eq!(primary_of(&[]), None);
}

#[test]
fn exactly_one_monitor_is_primary() {
    let env = environment(DisplayServer::Wayland, &[mon(Some("A"), (1, 1), (0, 0)), mon(Some("B"), (1, 1), (1, 0))]);
    assert_eq!(env.monitors.iter().filter(|m| m.primary).count(), 1);
}

#[test]
fn a_window_is_matched_to_its_monitor_by_position_and_size() {
    let raw = arrange(vec![mon(Some("DP-1"), (2560, 1440), (0, 0)), mon(Some("DP-2"), (1920, 1080), (2560, 0))]);
    assert_eq!(id_at(&raw, (2560, 0), (1920, 1080)).as_deref(), Some("DP-2"));
    assert_eq!(id_at(&raw, (5, 5), (1920, 1080)), None);
}

#[test]
fn logical_size_divides_by_the_scale_and_survives_a_zero() {
    let mut m = mon(None, (3840, 2160), (0, 0));
    m.scale = 2.;
    assert_eq!(m.logical_size(), (1920., 1080.));
    m.scale = 0.;
    assert_eq!(m.logical_size(), (3840., 2160.), "a bad scale is treated as 1");
}

#[test]
fn the_display_server_is_read_from_the_session_variables() {
    if cfg!(any(target_os = "windows", target_os = "macos")) {
        return;
    }
    assert_eq!(
        detect_server(vars(&[("XDG_SESSION_TYPE", "wayland"), ("WAYLAND_DISPLAY", "wayland-0"), ("DISPLAY", ":0")])),
        DisplayServer::Wayland
    );
    assert_eq!(detect_server(vars(&[("XDG_SESSION_TYPE", "x11"), ("DISPLAY", ":0")])), DisplayServer::X11);
    assert_eq!(detect_server(vars(&[("DISPLAY", ":1")])), DisplayServer::X11);
    assert_eq!(
        detect_server(vars(&[("GAMESCOPE_WAYLAND_DISPLAY", "gamescope-0"), ("WAYLAND_DISPLAY", "wayland-0")])),
        DisplayServer::Gamescope
    );
    assert_eq!(detect_server(vars(&[("XDG_CURRENT_DESKTOP", "gamescope")])), DisplayServer::Gamescope);
    assert_eq!(detect_server(vars(&[])), DisplayServer::Unknown);
}
