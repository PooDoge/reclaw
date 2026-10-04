//! Jumping back to pages visited lately: the desktop's Recent menu and Deck's Quick access list.
use reclaw_input::{Action, Direction};
use reclaw_ui::nav::Route;

use crate::common::*;

#[test]
fn the_desktop_recent_menu_lists_where_you_were_newest_first_and_jumps() {
    let mut s = Mount::desktop().start();
    s.open(Route::Game { id: 1 });
    s.open(Route::Downloads {});
    s.open(Route::Mods {});

    let count = |s: &Session, text: &str| s.labels().iter().filter(|l| l.as_str() == text).count();
    let mods_before = count(&s, "Mods");
    s.click_label("Recent");
    assert!(s.has_label("Starfall 64"), "a game is named by its title: {:?}", s.labels());
    assert!(s.has_label("Library"), "{:?}", s.labels());
    let (downloads, game) = (s.top_label_box("Downloads").expect("entry"), s.top_label_box("Starfall 64").expect("entry"));
    assert!(downloads.1 < game.1, "newest first");
    assert_eq!(count(&s, "Mods"), mods_before, "the page you are on is not offered");

    s.click_label("Starfall 64");
    assert!(s.on_game("Nintendo 64"), "{:?}", s.labels());
}

#[test]
fn with_nowhere_to_go_back_to_the_recent_button_does_nothing() {
    let mut s = Mount::desktop().start();
    let before = s.labels();
    s.click_label("Recent");
    assert_eq!(s.labels(), before, "nothing opened");
}

#[test]
fn deck_quick_access_lists_recent_pages_and_a_press_goes_there() {
    let mut s = Mount::deck().start();
    s.pad(Action::Confirm); // Starfall 64's page
    s.pad(Action::Back); // home again
    s.pad(Action::QuickAccess);
    assert!(s.has_label("RECENT"), "{:?}", s.labels());
    assert!(!s.has_label("Play"), "still on the home page");

    s.pad(Action::Navigate(Direction::Down)); // from Downloads to the first recent page
    s.pad(Action::Confirm);
    assert!(s.has_label("Play"), "on Starfall 64's page: {:?}", s.labels());
    s.snapshot("deck-recent-jump");
}

#[test]
fn deck_quick_access_has_no_recent_section_before_anything_was_visited() {
    let mut s = Mount::deck().start();
    s.pad(Action::QuickAccess);
    assert!(s.has_label("Quick access"));
    assert!(!s.has_label("RECENT"), "{:?}", s.labels());
}
