//! Key presses driving the real Deck mode component: what the reducer decides shows up on screen
//! and in the effects the host receives.

use crate::common::*;
use freya::prelude::NamedKey::*;
use reclaw_ui::{deck::TextField, effect::Effect, fixtures::sample_games};

/// Home -> Dino Rush (not installed) -> its game page -> Install page. Home's first row is
/// Starfall, Skyward Quest, Tide Racer; Down reaches the All apps shelf, where Dino Rush is fourth.
fn open_install(s: &mut Session) {
    s.presses(&[ArrowDown, ArrowRight, ArrowRight, ArrowRight, Enter, Enter]);
}

fn open_settings(s: &mut Session) {
    s.press(Tab);
    s.presses(&[ArrowDown, ArrowDown, ArrowDown, ArrowDown, Enter]);
}

#[test]
fn play_launches_the_focused_installed_app_and_navigation_alone_emits_nothing() {
    let mut s = Mount::deck().start();
    s.press(Enter); // open Starfall 64
    s.press(Enter); // Play
    assert_eq!(s.take_effects(), vec![Effect::Launch(1)]);

    s.press(Escape); // back to Home
    s.press(Tab); // main menu
    s.press(Escape); // close it
    assert_eq!(s.take_effects(), vec![], "navigation alone produces no effects");
}

#[test]
fn the_options_menu_opens_with_o_cascades_and_closes_with_back() {
    let mut s = Mount::deck().start();
    s.press_char("o");
    assert!(s.has_label("Manage"), "the menu is up: {:?}", s.labels());
    assert!(s.has_label("Starfall 64"), "titled with the app");

    s.presses(&[ArrowDown, Enter]); // Manage ...
    assert!(s.has_label("Open install folder"), "the submenu opened: {:?}", s.labels());

    s.press(Escape); // closes the submenu only
    assert!(!s.has_label("Open install folder"));
    assert!(s.has_label("Manage"));

    s.press(Escape); // closes the menu
    assert!(!s.has_label("Manage"));
    assert_eq!(s.take_effects(), vec![]);
}

#[test]
fn choosing_a_menu_row_sends_its_effect_and_uninstall_asks_first() {
    let mut s = Mount::deck().start();
    s.press_char("o");
    s.presses(&[ArrowDown, Enter, Enter]); // Manage -> Open install folder
    assert_eq!(s.take_effects(), vec![Effect::OpenFolder(1)]);
    assert!(!s.has_label("Manage"), "choosing closes the menu");

    s.press_char("o");
    s.presses(&[ArrowDown, Enter]); // Manage ->
    s.presses(&[ArrowDown, ArrowDown, ArrowDown, ArrowDown, Enter]); // -> Uninstall
    assert_eq!(s.take_effects(), vec![], "nothing is sent before the confirmation");
    assert!(s.has_label("Uninstall Starfall 64?"), "{:?}", s.labels());

    s.press(Enter); // Cancel is focused first
    assert_eq!(s.take_effects(), vec![]);
    assert!(!s.has_label("Uninstall Starfall 64?"));
}

#[test]
fn the_install_page_is_a_full_screen_page_with_back() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    assert!(s.has_label("Install Dino Rush"), "{:?}", s.labels());
    assert!(s.has_label("Back"), "the header Back button");

    s.press(Escape);
    assert!(!s.has_label("Install Dino Rush"), "Back returns to the game page");
}

#[test]
fn typing_into_the_install_location_is_committed_when_the_field_is_left() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    s.press(Enter); // focus is on the location field: start typing
    assert!(s.effects().contains(&Effect::BeginTextEntry(TextField::InstallLocation)), "{:?}", s.effects());

    s.type_text("/games");
    s.press(Enter); // done
    let committed = s.effects().into_iter().find_map(|e| match e {
        Effect::TextCommitted { app, field: TextField::InstallLocation, value } => Some((app, value)),
        _ => None,
    });
    let (app, value) = committed.expect("leaving the field reports what was typed");
    assert_eq!(app, Some(4));
    assert_eq!(value, "/games", "entering from the keyboard replaces the old text");
}

#[test]
fn leaving_a_field_empty_keeps_its_old_text() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    s.press(Enter); // start typing (the box is cleared)
    s.press(Enter); // done, nothing typed
    let value = s.effects().into_iter().find_map(|e| match e {
        Effect::TextCommitted { value, .. } => Some(value),
        _ => None,
    });
    assert_eq!(value.as_deref(), Some("~/Reclaw/Apps"), "an empty box gets its old text back");
}

#[test]
fn the_keyboard_hides_the_footer_and_scrolls_the_focused_row_into_view() {
    // A landscape handheld with an on-screen keyboard over its lower 45 percent.
    let mut s = Mount::deck().size(854., 480.).start();
    open_settings(&mut s);
    // Down to the Library section by its place in the schema, then drill in.
    let env = reclaw_games::settings::DisplayEnvironment::unknown();
    let schema = reclaw_ui::settings::global_settings(&reclaw_games::settings::all_specs(&env), &env);
    let library = schema.sections.iter().position(|section| section.id == "library").expect("the Library section");
    for _ in 0..library {
        s.press(ArrowDown);
    }
    s.press(Enter);
    assert!(s.has_label("Default install location") || s.has_label("Check for updates when Reclaw starts"), "{:?}", s.labels());
    // Down to the install location text row by its place in the section, then Enter: typing starts.
    let text_row = schema.sections[library].rows().position(|row| row.is_text()).expect("the text row");
    for _ in 0..text_row {
        s.press(ArrowDown);
    }
    s.press(Enter);
    assert!(s.effects().contains(&Effect::BeginTextEntry(TextField::DefaultLocation)));

    let (before_top, before_bottom) = s.label_span("Default install location").expect("the row is on screen");
    assert!(before_bottom > 480. - 216., "without a scroll the keyboard would cover it: {before_bottom}");

    s.set_keyboard(216.);
    let (top, bottom) = s.label_span("Default install location").expect("the row is on screen");
    assert!(top < before_top, "the page scrolled up to make room: {before_top} -> {top}");
    assert!(top >= 48., "below the compact header: {top}");
    assert!(bottom <= 480. - 216., "above the keyboard: {bottom}");
    assert!(!s.has_label("Done"), "the hint bar is hidden while the keyboard is up");

    s.set_keyboard(0.);
    assert!(s.has_label("Done"), "and returns with it (typing shows only Done)");
}

#[test]
fn a_footer_button_is_hidden_while_the_keyboard_is_up_on_the_install_page() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    assert!(s.has_label("Install"), "footer Install button");
    s.set_keyboard(360.);
    assert!(!s.has_label("Install"), "footer hidden while typing: {:?}", s.labels());
    assert!(s.has_label("Install location"), "the field stays");
    s.set_keyboard(0.);
    assert!(s.has_label("Install"));
}

#[test]
fn settings_open_as_a_page_and_a_choice_is_a_centered_menu() {
    let mut s = Mount::deck().start();
    open_settings(&mut s);
    assert!(s.has_label("Settings") && s.has_label("Back"));
    assert!(s.has_label("Interface"));

    s.press(ArrowRight); // into the rows
    s.press(Enter); // the Interface row opens its picker
    assert!(s.has_label("Desktop") && s.has_label("Deck"), "the picker lists the modes: {:?}", s.labels());
    s.presses(&[ArrowDown, Enter]); // Desktop
    let effects = s.take_effects();
    assert!(effects.contains(&Effect::SetMode(reclaw_ui::deck::ModePref::Desktop)), "{effects:?}");
}

#[test]
fn freyas_own_focus_keys_do_not_double_fire_a_press() {
    // Tab opens the main menu here, and Freya also treats it as "focus the next element". If a
    // button took that focus, the next Enter would press it as well as confirm.
    let mut s = Mount::deck().games(with_run(sample_games(), 1, running())).start();
    s.presses(&[Tab, Escape, Tab, Escape]);
    s.take_effects();
    s.press(Enter); // the Resume button in the banner has the pad's focus
    assert_eq!(s.take_effects(), vec![Effect::Resume(1), Effect::InputOwner(reclaw_input::InputOwner::App)]);
}

#[test]
fn freyas_focus_keys_do_not_pull_the_keyboard_out_of_a_text_box() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    s.press(Enter);
    s.type_text("ab");
    s.press(ArrowDown); // Freya's own focus keys must not pull the keyboard out of the box
    s.press(Tab);
    s.type_text("cd");
    s.press(Enter);
    let value = s.effects().into_iter().find_map(|e| match e {
        Effect::TextCommitted { value, .. } => Some(value),
        _ => None,
    });
    assert_eq!(value.as_deref(), Some("abcd"));
}

#[test]
fn text_typed_after_leaving_a_field_does_not_land_in_it() {
    let mut s = Mount::deck().start();
    open_install(&mut s);
    s.press(Enter);
    s.type_text("ab");
    s.press(Enter); // done: the page has the keyboard again
    s.type_text("zz");
    s.take_effects();
    s.press(Enter); // enter the box again
    s.press(Enter); // and leave it empty: the old text comes back
    let value = s.effects().into_iter().find_map(|e| match e {
        Effect::TextCommitted { value, .. } => Some(value),
        _ => None,
    });
    assert_eq!(value.as_deref(), Some("ab"), "the stray text went nowhere");
}
