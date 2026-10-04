//! Slide-in panels, the cascading Options menu, and confirmations.
use super::support::*;

#[test]
fn main_menu_traps_focus_and_navigates() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[MainMenu]);
    assert_eq!(s.focus(), ids::menu(0));
    press(&mut s, &f, &[go(Down), go(Down), Confirm]);
    assert_eq!((s.section(), s.overlay()), (Section::Downloads, Overlay::None));
    press(&mut s, &f, &[MainMenu]);
    assert!(ids::menu_index(s.focus()).is_some(), "a tile is not reachable from the menu: the trap held");
}

#[test]
fn switching_to_desktop_is_an_effect() {
    let f = Fixture::new();
    let mut s = f.state();
    let mut actions = vec![MainMenu];
    actions.extend([go(Down); 5]);
    actions.push(Confirm);
    assert_eq!(press(&mut s, &f, &actions), vec![Effect::SwitchToDesktop]);
}

#[test]
fn quick_access_shortcut_opens_downloads_and_cancel_is_an_effect() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[QuickAccess, Confirm]);
    assert_eq!(s.section(), Section::Downloads);
    assert_eq!(s.focus(), ids::download_cancel(1), "the first row in the queue, by activity id");
    assert_eq!(s.apply(Confirm, &f.view()), vec![Effect::CancelActivity(1)]);
}

#[test]
fn options_open_for_the_focused_tile() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options]);
    assert_eq!(s.overlay(), Overlay::Menu(MenuPurpose::Options(1)));
    let menu = s.menu().expect("a menu is open");
    assert_eq!(menu.levels()[0].title, "Starfall 64");
    assert_eq!(menu.levels().len(), 1);
}

#[test]
fn secondary_and_the_manage_button_open_the_same_menu() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Secondary]);
    assert_eq!(s.overlay(), Overlay::Menu(MenuPurpose::Options(1)));
    press(&mut s, &f, &[Back, Confirm, go(Down), go(Right), Confirm]);
    assert_eq!(s.screen(), Screen::Game(1));
    assert_eq!(s.focus(), ids::GAME_MANAGE);
    assert!(matches!(s.overlay(), Overlay::Menu(_)));
}

#[test]
fn the_add_to_submenu_cascades_and_a_choice_files_the_app() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Right)]);
    let menu = s.menu().unwrap();
    assert_eq!(menu.levels().len(), 2, "the parent stays visible beside the submenu");
    assert_eq!(menu.levels()[1].title, "Add to");
    let fx = press(&mut s, &f, &[Confirm]);
    assert_eq!(fx, vec![Effect::AddToCollection { app: 1, name: "Handheld friendly" }]);
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn back_closes_a_submenu_then_the_menu() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Right), Back]);
    assert_eq!(s.menu().unwrap().levels().len(), 1);
    press(&mut s, &f, &[Back]);
    assert_eq!(s.overlay(), Overlay::None);
    assert!(s.menu().is_none());
    assert_eq!(s.focus(), tile(0, 1), "focus is where it was");
}

#[test]
fn cancel_closes_without_an_effect() {
    let f = Fixture::new();
    let mut s = f.state();
    let mut actions = vec![Options];
    actions.extend([go(Down); 4]);
    actions.push(Confirm);
    assert!(press(&mut s, &f, &actions).is_empty());
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn pointer_pick_and_scrim_tap() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options]);
    assert_eq!(s.pick_menu(0, 0, &f.view()), vec![Effect::ToggleFavorite(1)]);
    press(&mut s, &f, &[Options]);
    assert!(s.dismiss(&f.view()).is_empty());
    assert_eq!(s.overlay(), Overlay::None);
    assert!(!s.focus_visible());
}

#[test]
fn manage_items_are_disabled_for_an_app_that_is_not_installed() {
    let f = Fixture::new();
    let mut s = f.state();
    s.focus = tile(1, 4); // Dino Rush: not installed
    press(&mut s, &f, &[Options]);
    assert!(s.menu().unwrap().levels()[0].entries[2].enabled, "the Manage row itself opens");
    s.pick_menu(0, 2, &f.view());
    let level = &s.menu().unwrap().levels()[1];
    assert!(level.entries.iter().all(|e| !e.enabled), "nothing to manage until it is installed");
    assert_eq!(s.pick_menu(1, 0, &f.view()), vec![], "a disabled row cannot be chosen");
}

#[test]
fn uninstall_asks_first_and_defaults_to_cancel() {
    let f = Fixture::new();
    let mut s = f.state();
    // Options > Manage > Uninstall (the last Manage row).
    press(&mut s, &f, &[Options, go(Down), go(Down), go(Right), go(Down), go(Down), go(Down), Confirm]);
    assert_eq!(s.overlay(), Overlay::Confirm(ConfirmKind::Uninstall(1)));
    assert_eq!(s.focus(), ids::CONFIRM_CANCEL, "the safe choice is under the thumb");

    assert!(press(&mut s, &f, &[Confirm]).is_empty());
    assert_eq!(s.overlay(), Overlay::None);

    press(&mut s, &f, &[Options, go(Down), go(Down), go(Right), go(Down), go(Down), go(Down), Confirm, go(Right)]);
    assert_eq!(s.focus(), ids::CONFIRM_OK);
    assert_eq!(press(&mut s, &f, &[Confirm]), vec![Effect::Uninstall(1)]);
    assert_eq!((s.screen(), s.overlay()), (Screen::Home, Overlay::None));
}

#[test]
fn panels_do_not_open_over_a_confirmation() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options, go(Down), go(Down), go(Right), go(Down), go(Down), go(Down), Confirm]);
    press(&mut s, &f, &[MainMenu, QuickAccess]);
    assert!(matches!(s.overlay(), Overlay::Confirm(_)));
}
