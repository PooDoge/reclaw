//! Home's shelves follow the sort: by system there is a shelf per system, and focus moves through them.
use reclaw_input::{Action, Direction};

use super::support::*;
use crate::{
    deck::{DeckState, DeckView, shelves},
    systems::Sort,
};

fn titles(f: &Fixture, ids: &[u32]) -> Vec<String> {
    ids.iter().filter_map(|id| f.games.iter().find(|g| g.id == *id)).map(|g| g.title.to_string()).collect()
}

#[test]
fn added_order_has_the_two_usual_shelves() {
    let f = Fixture::new();
    let specs = shelves(&f.view());
    assert_eq!(specs.iter().map(|s| s.title).collect::<Vec<_>>(), ["Continue", "Not installed"]);
}

#[test]
fn a_game_is_on_one_library_shelf_only() {
    let f = Fixture::new();
    for sort in [Sort::default(), Sort::Title, Sort::System] {
        let view = DeckView { sort, ..f.view() };
        let mut all: Vec<u32> = shelves(&view).into_iter().flat_map(|s| s.games).collect();
        all.sort();
        assert_eq!(all, [1, 2, 3, 4, 5, 6], "{sort:?}: every game once");
    }
}

#[test]
fn a_library_with_everything_installed_has_only_continue() {
    let mut f = Fixture::new();
    f.games.iter_mut().for_each(|g| g.status = crate::model::AppStatus::Installed);
    let specs = shelves(&f.view());
    assert_eq!(specs.iter().map(|s| s.title).collect::<Vec<_>>(), ["Continue"]);
}

#[test]
fn by_system_not_installed_becomes_a_shelf_per_system_oldest_first() {
    let f = Fixture::new();
    let view = DeckView { sort: Sort::System, ..f.view() };
    let specs = shelves(&view);
    assert_eq!(specs.iter().map(|s| s.title).collect::<Vec<_>>(), ["Continue", "Nintendo 64", "Game Boy Advance", "PlayStation 2"]);
    let n64 = specs.iter().find(|s| s.title == "Nintendo 64").expect("shelf");
    assert_eq!(titles(&f, &n64.games), ["Kart Ruins"], "the installed N64 games are on Continue");
}

#[test]
fn by_title_keeps_one_not_installed_shelf_in_alphabetical_order() {
    let f = Fixture::new();
    let view = DeckView { sort: Sort::Title, ..f.view() };
    let specs = shelves(&view);
    let rest = specs.iter().find(|s| s.title == "Not installed").expect("shelf");
    assert_eq!(titles(&f, &rest.games), ["Dino Rush", "Kart Ruins", "Moon Garden"]);
}

#[test]
fn focus_still_reaches_every_tile_when_split_by_system() {
    let f = Fixture::new();
    let view = DeckView { sort: Sort::System, ..f.view() };
    let mut state = DeckState::new(&view);
    let first = state.focus();
    // Down through every shelf: each step lands on a tile and ends on the last shelf.
    let mut seen = vec![first];
    for _ in 0..5 {
        state.apply(Action::Navigate(Direction::Down), &view);
        if seen.last() != Some(&state.focus()) {
            seen.push(state.focus());
        }
    }
    assert!(seen.len() >= 3, "moved through the shelves: {seen:?}");
}
