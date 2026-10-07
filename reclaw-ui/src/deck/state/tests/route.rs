//! Deck's page and the router's route stay in step: the reducer asks the router to move when it
//! moves, and follows when something else moves the router.
use super::support::*;
use crate::{deck::settings::SettingsTarget, nav::Route};

#[test]
fn opening_a_game_and_going_back_ask_the_router() {
    let f = Fixture::new();
    let mut s = f.state();
    assert_eq!(press_with_nav(&mut s, &f, &[Confirm]), vec![Effect::Navigate(Route::Game { id: 1 })]);
    assert_eq!(s.route(), Route::Game { id: 1 });
    assert_eq!(press_with_nav(&mut s, &f, &[Back]), vec![Effect::Back]);
    assert_eq!(s.route(), Route::Library {});
}

#[test]
fn the_bumpers_and_the_main_menu_switch_tabs_through_the_router() {
    let f = Fixture::new();
    let mut s = f.state();
    assert_eq!(press_with_nav(&mut s, &f, &[NextSection]), vec![Effect::Navigate(Route::Catalog {})]);
    assert_eq!(press_with_nav(&mut s, &f, &[NextSection]), vec![Effect::Navigate(Route::Downloads {})]);
    assert_eq!(press_with_nav(&mut s, &f, &[PrevSection]), vec![Effect::Navigate(Route::Catalog {})]);
    // The menu opens on the current tab (Catalog); two down is Mods.
    let menu = press_with_nav(&mut s, &f, &[MainMenu, go(Down), go(Down), Confirm]);
    assert_eq!(menu, vec![Effect::Navigate(Route::Mods {})]);
}

#[test]
fn settings_and_install_are_routes_too() {
    let f = Fixture::new();
    let mut s = f.state();
    let fx = press_with_nav(&mut s, &f, &[MainMenu, go(Down), go(Down), go(Down), go(Down), Confirm]);
    assert_eq!(fx, vec![Effect::Navigate(Route::Settings {})]);
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::Global));
    let mut s = f.state();
    s.click(tile(1, 3), &f.view());
    assert!(press_with_nav(&mut s, &f, &[Confirm]).contains(&Effect::Navigate(Route::Install { id: 3 })));
}

#[test]
fn following_the_router_shows_that_page_and_is_quiet_when_already_there() {
    let f = Fixture::new();
    let mut s = f.state();
    assert!(!s.follow(&Route::Library {}, &f.view()), "already there");
    assert!(s.follow(&Route::Game { id: 4 }, &f.view()));
    assert_eq!(s.screen(), Screen::Game(4));
    assert!(s.follow(&Route::Mods {}, &f.view()));
    assert_eq!((s.screen(), s.section()), (Screen::Home, Section::Mods));
    assert!(s.follow(&Route::GameSettings { id: 2 }, &f.view()));
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::App(2)));
    assert!(s.follow(&Route::SettingsSection { section: "motion".into() }, &f.view()));
    assert_eq!(s.screen(), Screen::Settings(SettingsTarget::Global));
}

#[test]
fn following_closes_a_menu_that_belonged_to_the_old_page() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Options]);
    assert_ne!(s.overlay(), Overlay::None);
    s.follow(&Route::Catalog {}, &f.view());
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn a_route_to_a_game_that_is_gone_leaves_deck_on_its_tab() {
    let f = Fixture::new();
    let mut s = f.state();
    s.follow(&Route::Catalog {}, &f.view());
    s.follow(&Route::Game { id: 404 }, &f.view());
    assert_eq!((s.screen(), s.section()), (Screen::Home, Section::Catalog));
}

#[test]
fn a_mod_page_shows_the_mods_tab_because_deck_has_no_page_for_one_mod() {
    let f = Fixture::new();
    let mut s = f.state();
    s.follow(&Route::ModDetail { provider: "thunderstore".into(), mod_id: "x".into() }, &f.view());
    assert_eq!((s.screen(), s.section()), (Screen::Home, Section::Mods));
}

#[test]
fn a_game_that_disappears_sends_the_router_home() {
    let mut f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Confirm]);
    f.games.retain(|g| g.id != 1);
    f.catalog.retain(|g| g.id != 1);
    let fx = s.sync(&f.view());
    assert!(fx.contains(&Effect::Navigate(Route::Library {})), "{fx:?}");
}
