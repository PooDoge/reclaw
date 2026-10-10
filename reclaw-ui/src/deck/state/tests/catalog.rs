//! The Catalog tab: a shelf per system that the pad moves through, and a catalog project that is not
//! in the library still opens its page. Also real-sized app ids, which are 32-bit hashes.
use super::support::*;
use crate::{deck::catalog_shelves, model::GameEntry};

/// A catalog project the user has not added, with an id like the real ones (any of the 32 bits set).
fn not_in_library(id: u32) -> GameEntry {
    let mut game = GameEntry::from_project(&crate::fixtures::sample_projects()[2]);
    game.id = id;
    game.title = "Hash Racer".into();
    game
}

fn on_catalog(f: &Fixture) -> DeckState {
    let mut s = f.state();
    press(&mut s, f, &[NextSection]);
    assert_eq!((s.screen(), s.section()), (Screen::Home, Section::Catalog));
    s
}

#[test]
fn the_catalog_tab_has_a_shelf_per_system_and_focus_starts_on_its_first_tile() {
    let f = Fixture::new();
    let titles: Vec<_> = catalog_shelves(&f.view()).iter().map(|s| s.title).collect();
    assert_eq!(titles, ["Nintendo 64", "Game Boy Advance", "PlayStation 2"]);
    let s = on_catalog(&f);
    let first = catalog_shelves(&f.view())[0].games[0];
    assert_eq!(s.focus(), tile(0, first));
}

#[test]
fn a_catalog_project_outside_the_library_opens_its_page_and_offers_to_install() {
    let mut f = Fixture::new();
    f.catalog.push(not_in_library(0xC0FF_EE01));
    let mut s = on_catalog(&f);
    let (shelf, _) = catalog_shelves(&f.view())
        .iter()
        .enumerate()
        .find_map(|(i, spec)| spec.games.contains(&0xC0FF_EE01).then_some((i, ())))
        .expect("listed");
    s.click(tile(shelf, 0xC0FF_EE01), &f.view());
    assert_eq!(s.screen(), Screen::Game(0xC0FF_EE01));
    press(&mut s, &f, &[Confirm]);
    assert_eq!(s.screen(), Screen::Install(0xC0FF_EE01), "not installed: the primary button installs");
}

#[test]
fn an_empty_catalog_has_nothing_to_focus() {
    let mut f = Fixture::new();
    f.catalog.clear();
    let s = on_catalog(&f);
    assert!(s.nodes(&f.view()).is_empty());
}

#[test]
fn real_app_ids_focus_one_tile_and_open_the_right_game() {
    // Two ids that differ only in bit 20, where the old encoding kept the shelf, and one with the top
    // bits set, where it kept the kind of target.
    let mut f = Fixture::new();
    let renumber = [(1, 0x9E37_79B9), (2, 0x9E27_79B9), (6, 0xF000_0006)];
    for (old, new) in renumber {
        f.games.iter_mut().filter(|g| g.id == old).for_each(|g| g.id = new);
        f.catalog.iter_mut().filter(|g| g.id == old).for_each(|g| g.id = new);
    }
    let mut s = f.state();
    let nodes = s.nodes(&f.view());
    let mut unique: Vec<_> = nodes.iter().map(|n| n.id).collect();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), nodes.len(), "every tile is its own target");
    for (_, id) in renumber {
        let shelf = shelves(&f.view()).iter().position(|spec| spec.games.contains(&id)).expect("on a shelf");
        s.click(tile(shelf, id), &f.view());
        assert_eq!(s.screen(), Screen::Game(id), "{id:#x}");
        press(&mut s, &f, &[Back]);
    }
}
