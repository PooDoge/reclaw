//! The window around the UI: the custom title bar and what its buttons ask for, and the Screen
//! settings that describe the monitors and where Deck mode appears.
//!
//! No real window stands behind these tests, so a button press shows up as an `Effect::Window` for
//! the host rather than as a minimized window.
use reclaw_games::settings::DisplayServer;
use reclaw_ui::{
    effect::Effect,
    nav::Route,
    settings::{SettingChange, SettingValue, SettingsTarget},
    store::AppAction,
    window::{Edge, Frame, RawMonitor, WindowCommand, environment},
};

use crate::common::*;

fn monitor(name: &str, size: (u32, u32), x: i32, refresh_mhz: u32) -> RawMonitor {
    RawMonitor { name: Some(name.into()), size, position: (x, 0), scale: 1., refresh_mhz: Some(refresh_mhz), primary: false }
}

/// The middle of the nth button from the right edge (0 is Close), on the title bar's row.
fn button_from_right(s: &Session, n: usize) -> (f64, f64) {
    (f64::from(s.size.0) - 23. - 46. * n as f64, 18.)
}

fn click(s: &mut Session, at: (f64, f64)) {
    s.runner.click_cursor(at);
    s.settle();
}

#[test]
fn without_the_custom_frame_the_desktop_draws_no_title_bar() {
    let mut s = Mount::desktop().start();
    let at = button_from_right(&s, 0);
    click(&mut s, at);
    assert!(!s.effects().iter().any(|e| matches!(e, Effect::Window(_))), "{:?}", s.effects());
}

#[test]
fn the_title_bar_has_minimize_maximize_and_close() {
    let mut s = Mount::desktop().frame(Frame::Custom).start();
    for (n, command) in [(2, WindowCommand::Minimize), (1, WindowCommand::ToggleMaximize), (0, WindowCommand::Close)] {
        let at = button_from_right(&s, n);
        click(&mut s, at);
        assert_eq!(s.take_effects(), vec![Effect::Window(command.clone())], "button {n} should ask for {command:?}");
    }
    s.snapshot("desktop-titlebar");
}

#[test]
fn deck_mode_has_no_title_bar() {
    let mut s = Mount::deck().frame(Frame::Custom).start();
    let at = button_from_right(&s, 0);
    click(&mut s, at);
    assert!(!s.effects().iter().any(|e| matches!(e, Effect::Window(_))), "Deck mode fills the screen: {:?}", s.effects());
}

#[test]
fn the_title_bar_goes_with_the_interface_when_switching_mode() {
    let mut s = Mount::desktop().frame(Frame::Custom).start();
    s.press(freya::prelude::NamedKey::F10); // to Deck mode
    let at = button_from_right(&s, 0);
    click(&mut s, at);
    assert!(!s.effects().iter().any(|e| matches!(e, Effect::Window(_))), "{:?}", s.effects());
}

fn two_monitors() -> reclaw_games::settings::DisplayEnvironment {
    environment(DisplayServer::Wayland, &[monitor("HDMI-A-1", (1920, 1080), 2560, 60_000), monitor("DP-1", (2560, 1440), 0, 143_912)])
}

#[test]
fn the_screen_section_lists_the_monitors_in_left_to_right_order() {
    let mut s = Mount::desktop().start_at(Route::Settings {});
    s.dispatch(AppAction::SetDisplay(two_monitors()));
    s.click_label("Screen");
    for label in ["Fill the screen in Deck mode", "Deck mode monitor", "Display server", "Wayland", "Monitor 1", "Monitor 2"] {
        assert!(s.has_label(label), "{label}: {:?}", s.labels());
    }
    let (first, second) = (s.label_span("Monitor 1").expect("row"), s.label_span("Monitor 2").expect("row"));
    assert!(first.0 < second.0, "Monitor 1 is listed first");
    assert!(s.has_label("DP-1, 2560x1440, 144 Hz"), "the leftmost, with its refresh rate rounded: {:?}", s.labels());
    assert!(s.has_label("HDMI-A-1, 1920x1080, 60 Hz"), "{:?}", s.labels());
    s.snapshot("desktop-settings-screen");
}

#[test]
fn a_session_with_no_monitors_known_still_has_the_screen_rows() {
    let mut s = Mount::desktop().start_at(Route::Settings {});
    s.click_label("Screen");
    assert!(s.has_label("Fill the screen in Deck mode"), "{:?}", s.labels());
    assert!(s.has_label("Unknown"), "the display server row says it does not know: {:?}", s.labels());
    assert!(!s.has_label("Monitor 1"));
}

#[test]
fn turning_off_deck_fullscreen_is_saved_and_reported() {
    let mut s = Mount::desktop().start_at(Route::Settings {});
    s.click_label("Screen");
    s.click_label("Fill the screen in Deck mode");
    assert_eq!(
        s.take_effects(),
        vec![Effect::Setting(SettingChange { app: None, key: "deck_fullscreen", value: SettingValue::Bool(false) })]
    );
    assert!(!s.store().snapshot().settings.toggle(SettingsTarget::Global, "deck_fullscreen", true));
}

#[test]
fn choosing_a_monitor_for_deck_mode_saves_its_number() {
    let mut s = Mount::desktop().start_at(Route::Settings {});
    s.dispatch(AppAction::SetDisplay(two_monitors()));
    s.click_label("Screen");
    s.click_label("Deck mode monitor");
    s.click_label("Monitor 2");
    assert_eq!(s.store().snapshot().settings.choice(SettingsTarget::Global, "deck_display", 0), 2);
}

/// Press `at` (physical pixels) and say what the window was asked to do.
fn pressed(s: &mut Session, at: (f64, f64)) -> Vec<Effect> {
    click(s, at);
    s.take_effects()
}

#[test]
fn the_edges_of_the_window_start_a_resize_from_that_edge() {
    let mut s = Mount::desktop().frame(Frame::Custom).size(1280., 800.).start();
    let (w, h) = (1280., 800.);
    for (at, edge) in [
        ((w - 3., h / 2.), Edge::East),
        ((w / 2., h - 3.), Edge::South),
        ((3., h / 2.), Edge::West),
        ((w / 2., 3.), Edge::North),
        ((w - 3., h - 3.), Edge::SouthEast),
        ((3., 3.), Edge::NorthWest),
        ((w - 3., 3.), Edge::NorthEast),
        ((3., h - 3.), Edge::SouthWest),
    ] {
        assert_eq!(pressed(&mut s, at), vec![Effect::Window(WindowCommand::BeginResize(edge))], "pressing at {at:?}");
    }
}

/// The failure that only showed on a scaled display: Freya's bands were placed from the window's physical size, so
/// at 150% the right and bottom bands lay outside the window and only the left and top edges answered.
#[test]
fn the_right_and_bottom_edges_answer_on_a_scaled_display_too() {
    for scale in [1.25, 1.5, 2.] {
        let (w, h) = (1280. * scale, 800. * scale);
        let mut s = Mount::desktop().frame(Frame::Custom).scale(scale).size(w as f32, h as f32).start();
        for (at, edge) in [((w - 4., h / 2.), Edge::East), ((w / 2., h - 4.), Edge::South), ((w - 4., h - 4.), Edge::SouthEast)] {
            assert_eq!(pressed(&mut s, at), vec![Effect::Window(WindowCommand::BeginResize(edge))], "at {scale}x, pressing {at:?}");
        }
        // Well inside the window nothing starts a resize.
        assert!(pressed(&mut s, (w - 60., h / 2.)).iter().all(|e| !matches!(e, Effect::Window(_))), "at {scale}x");
    }
}

#[test]
fn deck_mode_and_the_native_frame_have_no_resize_bands_of_ours() {
    let mut native = Mount::desktop().frame(Frame::Native).start();
    let at = (f64::from(native.size.0) - 3., 300.);
    assert!(!pressed(&mut native, at).iter().any(|e| matches!(e, Effect::Window(WindowCommand::BeginResize(_)))));
}
