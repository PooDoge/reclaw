//! Pages, history and transitions through the real router and the real desktop pages.

use std::time::Duration;

use crate::common::*;
use freya::prelude::*;
use freya_testing::prelude::*;
use reclaw_ui::{
    nav::Route,
    shell::{DevOverrides, MotionOverride},
};

fn motion(m: MotionOverride) -> DevOverrides {
    DevOverrides { motion: Some(m), ..DevOverrides::default() }
}

/// Markers that identify a page without depending on its layout: the Library has the Favorites
/// chip; a Game page lists its platform; Settings is still a stand-in.
impl Session {
    fn on_library(&self) -> bool {
        self.has_label("Favorites")
    }

    fn on_game(&self, platform: &str) -> bool {
        self.has_label("Back") && self.has_label(platform)
    }

    fn open(&mut self, route: Route) {
        self.open_route(route);
        self.pump(30);
        self.runner.sync_and_update();
    }

    fn mouse_button(&mut self, button: MouseButton) {
        self.runner.send_event(PlatformEvent::Mouse { name: MouseEventName::MouseUp, cursor: (100., 100.).into(), button: Some(button) });
        self.pump(30);
        self.runner.sync_and_update();
    }

    fn alt(&mut self, key: NamedKey) {
        self.runner.send_event(PlatformEvent::Keyboard {
            name: KeyboardEventName::KeyDown,
            key: Key::Named(key),
            code: Code::Unidentified,
            modifiers: Modifiers::ALT,
        });
        self.pump(30);
        self.runner.sync_and_update();
    }
}

#[test]
fn the_host_can_open_a_page_and_the_old_one_is_gone_once_it_settles() {
    let mut s = Mount::desktop().start();
    assert!(s.on_library());
    s.open(Route::Game { id: 4 });
    s.pump(600);
    s.runner.sync_and_update();
    assert!(s.on_game("PlayStation 2"), "{:?}", s.labels());
    assert!(!s.on_library(), "the old page is dropped after the transition: {:?}", s.labels());
}

#[test]
fn during_a_transition_both_pages_are_there() {
    let mut s = Mount::desktop().start();
    s.open(Route::Game { id: 4 });
    assert!(s.on_game("PlayStation 2") && s.on_library(), "{:?}", s.labels());
}

#[test]
fn reduced_motion_cuts_straight_to_the_new_page() {
    let mut s = Mount::desktop().dev(motion(MotionOverride::Reduced)).start();
    s.open(Route::Game { id: 4 });
    assert!(s.on_game("PlayStation 2") && !s.on_library(), "{:?}", s.labels());
}

#[test]
fn the_mouse_side_buttons_walk_the_history() {
    let mut s = Mount::desktop().dev(motion(MotionOverride::Reduced)).start();
    s.open(Route::Game { id: 4 });
    s.open(Route::Game { id: 5 });
    s.mouse_button(MouseButton::Back);
    assert!(s.on_game("PlayStation 2"), "{:?}", s.labels());
    s.mouse_button(MouseButton::Back);
    assert!(s.on_library(), "{:?}", s.labels());
    s.mouse_button(MouseButton::Back);
    assert!(s.on_library(), "nowhere further back: {:?}", s.labels());
    s.mouse_button(MouseButton::Forward);
    s.mouse_button(MouseButton::Forward);
    assert!(s.on_game("Game Boy Advance"), "{:?}", s.labels());
}

#[test]
fn alt_arrows_do_the_same() {
    let mut s = Mount::desktop().dev(motion(MotionOverride::Reduced)).start();
    s.open(Route::Game { id: 4 });
    s.alt(NamedKey::ArrowLeft);
    assert!(s.on_library());
    s.alt(NamedKey::ArrowRight);
    assert!(s.on_game("PlayStation 2"));
}

#[test]
fn a_page_opened_by_a_link_goes_up_when_there_is_no_history() {
    let mut s = Mount::desktop().dev(motion(MotionOverride::Reduced)).start_at(Route::Install { id: 4 });
    assert!(s.has_label("Install Dino Rush"), "the form is open over the game: {:?}", s.labels());
    s.mouse_button(MouseButton::Back);
    assert!(s.on_game("PlayStation 2") && !s.has_label("Install Dino Rush"), "up from the install form: {:?}", s.labels());
    s.mouse_button(MouseButton::Back);
    assert!(s.on_library(), "{:?}", s.labels());
}

#[test]
fn a_transition_ends_on_its_own() {
    let mut s = Mount::desktop().start();
    s.open(Route::Settings {});
    s.runner.poll(Duration::from_millis(16), Duration::from_millis(900));
    s.runner.sync_and_update();
    assert!(s.has_label("SettingsPage") && !s.on_library(), "{:?}", s.labels());
}

#[test]
fn back_and_escape_close_a_dialog_before_they_leave_the_page() {
    let mut s = Mount::desktop().dev(motion(MotionOverride::Reduced)).start_at(Route::Game { id: 4 });
    s.click_label("Install");
    assert!(s.has_label("Install Dino Rush"), "{:?}", s.labels());
    s.mouse_button(MouseButton::Back);
    assert!(!s.has_label("Install Dino Rush"), "the first Back closes the form: {:?}", s.labels());
    assert!(s.on_game("PlayStation 2"), "and stays on the page: {:?}", s.labels());

    s.click_label("Install");
    s.press(NamedKey::Escape);
    assert!(!s.has_label("Install Dino Rush") && s.on_game("PlayStation 2"), "Escape does the same: {:?}", s.labels());
}
