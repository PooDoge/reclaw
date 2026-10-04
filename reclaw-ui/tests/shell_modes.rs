//! Entering and leaving Deck mode by hand: the F10 key, the desktop's Deck mode button, the Deck
//! main menu, and the Interface setting. All of it works without a gamepad or a console session.
mod common;

use common::*;
use freya::prelude::NamedKey::*;
use reclaw_input::Action;
use reclaw_ui::{
    deck::ModePref,
    effect::Effect,
    shell::{DevOverrides, KeyboardStart},
};

const DESKTOP_MARK: &str = "Library synced"; // the desktop's status bar
const DECK_MARK: &str = "Continue"; // Deck home's first shelf

fn in_desktop(s: &Session) -> bool {
    s.has_label(DESKTOP_MARK) && !s.has_label(DECK_MARK)
}

fn in_deck(s: &Session) -> bool {
    s.has_label(DECK_MARK) && !s.has_label(DESKTOP_MARK)
}

#[test]
fn f10_switches_between_the_interfaces_and_back() {
    let mut s = Mount::desktop().start();
    assert!(in_desktop(&s), "{:?}", s.labels());
    s.press(F10);
    assert!(in_deck(&s), "{:?}", s.labels());
    s.press(F10);
    assert!(in_desktop(&s), "{:?}", s.labels());
}

#[test]
fn the_desktop_has_a_deck_mode_button() {
    let mut s = Mount::desktop().start();
    s.click_label("Deck mode");
    assert!(in_deck(&s), "{:?}", s.labels());
    s.snapshot("shell-entered-deck-from-desktop");
}

#[test]
fn the_deck_main_menu_returns_to_the_desktop() {
    let mut s = Mount::deck().start();
    s.press(Tab);
    s.presses(&[ArrowDown, ArrowDown, ArrowDown, ArrowDown, ArrowDown, Enter]); // Switch to desktop
    assert!(in_desktop(&s), "{:?}", s.labels());
    assert!(s.effects().contains(&Effect::SwitchToDesktop), "the host hears about it too");
}

#[test]
fn the_interface_setting_picks_the_mode() {
    let mut s = Mount::deck().start();
    s.press(Tab);
    s.presses(&[ArrowDown, ArrowDown, ArrowDown, ArrowDown, Enter]); // Settings
    s.presses(&[ArrowRight, Enter, ArrowDown, Enter]); // Interface -> Desktop
    assert!(s.effects().contains(&Effect::SetMode(ModePref::Desktop)));
    assert!(in_desktop(&s), "{:?}", s.labels());
}

#[test]
fn the_pad_still_works_after_leaving_and_re_entering_deck_mode() {
    let mut s = Mount::deck().start();
    s.press(F10); // desktop
    s.pad(Action::Confirm); // pressed while nothing listens: must not replay later
    s.press(F10); // deck again
    assert!(in_deck(&s));
    assert!(!s.has_label("Play"), "the stale press was dropped; still on Home");

    s.pad(Action::Confirm);
    assert!(s.has_label("Play"), "the pad opens the focused game after the round trip: {:?}", s.labels());
}

#[test]
fn f9_shows_a_simulated_keyboard_in_either_interface() {
    let mut s = Mount::deck().start();
    s.presses(&[ArrowDown, ArrowRight, ArrowRight, ArrowRight, Enter, Enter]); // Install page
    assert!(s.has_label("Install"));
    s.press(F9);
    assert!(!s.has_label("Install"), "the footer hid for the simulated keyboard: {:?}", s.labels());
    s.press(F9);
    assert!(s.has_label("Install"));
}

#[test]
fn the_simulated_keyboard_follows_text_focus_when_asked_to() {
    let dev = DevOverrides { keyboard_follows_focus: true, ..DevOverrides::default() };
    let mut s = Mount::deck().dev(dev).start();
    s.presses(&[ArrowDown, ArrowRight, ArrowRight, ArrowRight, Enter, Enter]);
    assert!(s.has_label("Install"));
    s.press(Enter); // start typing in the location box
    assert!(!s.has_label("Install"), "the keyboard came up with the text focus");
    s.press(Enter); // done
    assert!(s.has_label("Install"), "and went away with it");
}

#[test]
fn the_environment_can_start_with_the_keyboard_up() {
    let dev = DevOverrides { keyboard: Some(KeyboardStart::Pixels(300.)), ..DevOverrides::default() };
    let mut s = Mount::deck().dev(dev).start();
    s.presses(&[ArrowDown, ArrowRight, ArrowRight, ArrowRight, Enter, Enter]);
    assert!(!s.has_label("Install"), "footer hidden from the start");
}
