//! Deck mode on the router: B is the router's back, the mouse's back button and a link move the
//! page, and the page the app is on survives a switch of interface.
use freya::prelude::*;
use reclaw_input::Action;
use reclaw_ui::nav::Route;

use crate::common::*;

/// Deck's Library: its shelves.
fn on_home(s: &Session) -> bool {
    s.has_label("All apps")
}

#[test]
fn a_press_opens_a_game_and_b_goes_back_through_the_router() {
    let mut s = Mount::deck().start();
    assert!(on_home(&s));
    s.pad(Action::Confirm);
    assert!(!on_home(&s) && s.has_label("Starfall 64"), "the game page: {:?}", s.labels());
    s.pad(Action::Back);
    assert!(on_home(&s), "B returns: {:?}", s.labels());
}

#[test]
fn the_bumpers_switch_tabs_without_growing_the_back_stack() {
    let mut s = Mount::deck().start();
    s.pad(Action::NextSection);
    assert!(s.has_label("Community app lists will appear here."), "Catalog: {:?}", s.labels());
    // Tabs replace each other, so there is nothing behind the Catalog to go back to.
    s.mouse_button(MouseButton::Back);
    assert!(s.has_label("Community app lists will appear here."), "{:?}", s.labels());
    s.pad(Action::PrevSection);
    assert!(on_home(&s), "{:?}", s.labels());
}

#[test]
fn the_mouse_back_button_moves_deck_and_the_reducer_follows() {
    let mut s = Mount::deck().start();
    s.pad(Action::Confirm);
    s.mouse_button(MouseButton::Back);
    assert!(on_home(&s), "{:?}", s.labels());
    // The reducer followed: Confirm opens the focused game again rather than doing nothing.
    s.pad(Action::Confirm);
    assert!(!on_home(&s), "{:?}", s.labels());
}

#[test]
fn a_link_opens_deck_on_that_page() {
    let s = Mount::deck().start_at(Route::Game { id: 4 });
    assert!(s.has_label("Dino Rush") && !on_home(&s), "{:?}", s.labels());
    let s = Mount::deck().start_at(Route::Settings {});
    assert!(s.has_label("Interface") && s.has_label("Motion"), "{:?}", s.labels());
}

#[test]
fn the_page_survives_a_switch_to_the_desktop_and_back() {
    let mut s = Mount::deck().start_at(Route::Game { id: 4 });
    s.press(NamedKey::F10);
    assert!(s.has_label("Back") && s.has_label("Install"), "the desktop's game page: {:?}", s.labels());
    s.press(NamedKey::F10);
    assert!(s.has_label("Dino Rush") && !on_home(&s), "and Deck's again: {:?}", s.labels());
}

#[test]
fn deck_pages_move_with_the_console_transition_settings() {
    let mut s = Mount::deck().animated().start();
    s.pad(Action::Confirm);
    // Mid-transition both pages are drawn: the shelves leaving, the game arriving.
    assert!(on_home(&s) && s.has_label("Open folder"), "{:?}", s.labels());
    s.settle();
    assert!(!on_home(&s) && s.has_label("Open folder"), "only the game page remains: {:?}", s.labels());
}
