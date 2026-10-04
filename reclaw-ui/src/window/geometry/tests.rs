use super::*;

fn mon(size: (u32, u32), position: (i32, i32), scale: f64) -> RawMonitor {
    RawMonitor { name: None, size, position, scale, refresh_mhz: None, primary: false }
}

fn prefs(size: Option<(f32, f32)>, position: Option<(i32, i32)>) -> WindowPrefs {
    WindowPrefs { size, position, ..WindowPrefs::default() }
}

#[test]
fn a_saved_size_is_kept_rounded_and_within_limits() {
    assert_eq!(sane_size(Some((1000.4, 700.6))), (1000., 701.));
    assert_eq!(sane_size(Some((100., 50.))), MIN_SIZE, "too small to use");
    assert_eq!(sane_size(Some((1e9, 1e9))), (16_384., 16_384.));
}

#[test]
fn a_missing_or_broken_size_is_the_default() {
    for bad in [None, Some((f32::NAN, 800.)), Some((800., f32::INFINITY)), Some((0., 800.)), Some((-5., -5.))] {
        assert_eq!(sane_size(bad), DEFAULT_SIZE, "{bad:?}");
    }
}

#[test]
fn a_window_bigger_than_every_monitor_is_brought_down_to_the_biggest() {
    let monitors = [mon((1920, 1080), (0, 0), 1.), mon((1280, 720), (1920, 0), 1.)];
    assert_eq!(restore(&prefs(Some((3000., 2000.)), None), DisplayServer::Wayland, &monitors).size, Some((1920., 1080.)));
    assert_eq!(restore(&prefs(Some((1500., 900.)), None), DisplayServer::Wayland, &monitors).size, None, "it fits: leave it");
}

#[test]
fn the_fit_uses_logical_pixels() {
    let hidpi = [mon((3840, 2160), (0, 0), 2.)];
    assert_eq!(restore(&prefs(Some((3000., 1000.)), None), DisplayServer::Wayland, &hidpi).size, Some((1920., 1000.)));
}

#[test]
fn with_no_monitors_known_nothing_is_changed() {
    let r = restore(&prefs(Some((3000., 2000.)), Some((10, 10))), DisplayServer::X11, &[]);
    assert_eq!((r.size, r.position), (None, None));
}

#[test]
fn a_position_is_used_only_where_programs_may_place_windows() {
    let monitors = [mon((1920, 1080), (0, 0), 1.)];
    let saved = prefs(Some((1000., 700.)), Some((200, 100)));
    assert_eq!(restore(&saved, DisplayServer::X11, &monitors).position, Some((200, 100)));
    assert_eq!(restore(&saved, DisplayServer::Windows, &monitors).position, Some((200, 100)));
    for server in [DisplayServer::Wayland, DisplayServer::Gamescope, DisplayServer::Unknown] {
        assert_eq!(restore(&saved, server, &monitors).position, None, "{server:?}");
    }
}

#[test]
fn a_position_on_a_monitor_that_is_gone_is_dropped() {
    let one = [mon((1920, 1080), (0, 0), 1.)];
    let on_second = prefs(Some((1000., 700.)), Some((2500, 100)));
    assert_eq!(restore(&on_second, DisplayServer::X11, &one).position, None, "the second monitor was unplugged");
    let two = [mon((1920, 1080), (0, 0), 1.), mon((1920, 1080), (1920, 0), 1.)];
    assert_eq!(restore(&on_second, DisplayServer::X11, &two).position, Some((2500, 100)));
}

#[test]
fn only_a_sliver_on_screen_is_not_enough_but_a_grabbable_bar_is() {
    let monitors = [mon((1920, 1080), (0, 0), 1.)];
    let at = |x, y| restore(&prefs(Some((1000., 700.)), Some((x, y))), DisplayServer::X11, &monitors).position;
    assert_eq!(at(-950, 100), None, "50 px of the bar showing");
    assert_eq!(at(-880, 100), Some((-880, 100)), "120 px of it showing");
    assert_eq!(at(100, 1070), None, "the bar is below the screen");
    assert_eq!(at(100, -400), None, "the bar is above it");
}

#[test]
fn maximized_is_carried_over() {
    let saved = WindowPrefs { maximized: true, ..WindowPrefs::default() };
    assert!(restore(&saved, DisplayServer::Wayland, &[]).maximized);
}

fn now(size: (f32, f32)) -> Snapshot {
    Snapshot { size, position: Some((10, 20)), maximized: false, fullscreen: false, monitor: Some("DP-1".into()) }
}

#[test]
fn a_moved_or_resized_window_is_remembered() {
    let saved = remember(&WindowPrefs::default(), &now((1000.3, 700.))).expect("changed");
    assert_eq!(saved.size, Some((1000., 700.)));
    assert_eq!(saved.position, Some((10, 20)));
    assert_eq!(saved.monitor.as_deref(), Some("DP-1"));
}

#[test]
fn seeing_the_same_window_again_saves_nothing() {
    let saved = remember(&WindowPrefs::default(), &now((1000., 700.))).expect("first time");
    assert_eq!(remember(&saved, &now((1000.2, 700.))), None, "a fraction of a pixel is not a change");
}

#[test]
fn maximizing_keeps_the_restored_size() {
    let saved = remember(&WindowPrefs::default(), &now((1000., 700.))).expect("changed");
    let maximized = Snapshot { size: (1920., 1080.), position: Some((0, 0)), maximized: true, ..now((1920., 1080.)) };
    let after = remember(&saved, &maximized).expect("the flag changed");
    assert!(after.maximized);
    assert_eq!((after.size, after.position), (Some((1000., 700.)), Some((10, 20))), "un-maximizing returns to the old window");
}

#[test]
fn fullscreen_changes_nothing() {
    let saved = remember(&WindowPrefs::default(), &now((1000., 700.))).expect("changed");
    let deck = Snapshot { size: (2560., 1440.), position: Some((0, 0)), maximized: false, fullscreen: true, monitor: Some("DP-2".into()) };
    assert_eq!(remember(&saved, &deck), None);
}

#[test]
fn a_window_that_cannot_report_its_position_keeps_the_saved_one() {
    let saved = remember(&WindowPrefs::default(), &now((1000., 700.))).expect("changed");
    let wayland = Snapshot { position: None, ..now((1100., 700.)) };
    let after = remember(&saved, &wayland).expect("the size changed");
    assert_eq!(after.position, Some((10, 20)));
}
