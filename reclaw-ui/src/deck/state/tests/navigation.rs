use super::support::*;

#[test]
fn starts_on_the_first_tile() {
    let f = Fixture::new();
    let s = f.state();
    assert_eq!(s.focus(), tile(0, 1));
    assert_eq!((s.screen(), s.overlay(), s.section()), (Screen::Home, Overlay::None, Section::Library));
}

#[test]
fn moves_along_a_shelf_and_down_to_the_next() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[go(Right)]);
    assert_eq!(s.focus(), tile(0, 2));
    press(&mut s, &f, &[go(Down)]);
    // Continue holds 1, 2 and 6; "Not installed" holds 3, 4 and 5, so under the second tile is game 4.
    assert_eq!((ids::tile_shelf(s.focus()), ids::tile_game(s.focus())), (Some(1), Some(4)));
    press(&mut s, &f, &[go(Up)]);
    assert_eq!(s.focus(), tile(0, 2));
}

#[test]
fn edges_do_not_wrap() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[go(Left), go(Up)]);
    assert_eq!(s.focus(), tile(0, 1));
}

#[test]
fn confirm_opens_the_game_and_back_restores_focus() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[go(Right), Confirm]);
    assert_eq!(s.screen(), Screen::Game(2));
    assert_eq!(s.focus(), ids::GAME_PRIMARY);
    press(&mut s, &f, &[Back]);
    assert_eq!(s.screen(), Screen::Home);
    assert_eq!(s.focus(), tile(0, 2), "returns to the tile it left");
}

#[test]
fn back_on_home_does_nothing() {
    let f = Fixture::new();
    let mut s = f.state();
    assert!(s.apply(Back, &f.view()).is_empty());
    assert_eq!(s.screen(), Screen::Home);
}

#[test]
fn bumpers_cycle_sections_and_remember_focus() {
    let f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[go(Right), NextSection]);
    assert_eq!(s.section(), Section::Catalog);
    press(&mut s, &f, &[PrevSection]);
    assert_eq!((s.section(), s.focus()), (Section::Library, tile(0, 2)));
    press(&mut s, &f, &[PrevSection]);
    assert_eq!(s.section(), Section::Mods, "wraps around the tabs");
}

#[test]
fn pointer_click_focuses_and_confirms_and_hides_the_ring() {
    let f = Fixture::new();
    let mut s = f.state();
    assert!(s.focus_visible());
    s.click(tile(1, 3), &f.view());
    assert_eq!(s.screen(), Screen::Game(3));
    assert!(!s.focus_visible(), "mouse use hides the focus ring");
    s.set_last_input(LastInput::Gamepad(reclaw_input::ControllerKind::Xbox));
    assert!(s.focus_visible());
}

#[test]
fn focus_repairs_itself_when_its_target_disappears() {
    let mut f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[go(Right)]);
    f.games.retain(|g| g.id != 2);
    s.sync(&f.view());
    assert!(s.nodes(&f.view()).iter().any(|n| n.id == s.focus()));
}

#[test]
fn a_page_for_a_vanished_app_falls_back_home() {
    let mut f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.screen(), Screen::Game(1));
    f.games.clear();
    f.catalog.clear();
    s.sync(&f.view());
    assert_eq!(s.screen(), Screen::Home);
}

#[test]
fn a_page_for_an_app_removed_from_the_library_stays_while_the_catalog_lists_it() {
    let mut f = Fixture::new();
    let mut s = f.state();
    press(&mut s, &f, &[Confirm]);
    f.games.retain(|g| g.id != 1);
    s.sync(&f.view());
    assert_eq!(s.screen(), Screen::Game(1));
}
