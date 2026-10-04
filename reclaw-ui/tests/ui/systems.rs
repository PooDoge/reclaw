//! Browsing by system: the badge on cards and rows, the System filter and the Sort control in the
//! Library, the Catalog's system chips, and Deck mode's shelves per system.
use reclaw_ui::{
    effect::Effect,
    nav::Route,
    settings::{SettingChange, SettingValue, SettingsTarget},
    store::AppAction,
};

use crate::common::*;

fn top(s: &Session, title: &str) -> f32 {
    s.label_span(title).unwrap_or_else(|| panic!("{title:?} is not on screen: {:?}", s.labels())).0
}

#[test]
fn rows_and_cards_carry_the_mark_of_their_system() {
    let s = Mount::desktop().start();
    for mark in ["N64", "GBA", "PS2"] {
        assert!(s.has_label(mark), "{mark}: {:?}", s.labels());
    }
}

#[test]
fn the_system_chip_lists_the_systems_with_counts_and_narrows_the_list() {
    let mut s = Mount::desktop().start();
    assert!(s.has_label("All systems"));
    s.click_label("All systems");
    for entry in ["Nintendo 64 (4)", "Game Boy Advance (1)", "PlayStation 2 (1)"] {
        assert!(s.has_label(entry), "{entry}: {:?}", s.labels());
    }
    s.click_label("PlayStation 2 (1)");
    assert!(s.has_label("PlayStation 2"), "the chip names the system chosen");
    assert!(s.has_label("Dino Rush") && !s.has_label("Starfall 64"), "only that system's games: {:?}", s.labels());

    s.click_label("PlayStation 2");
    s.click_label("All systems");
    assert!(s.has_label("Starfall 64"), "back to everything");
}

#[test]
fn sorting_by_system_reorders_the_list_and_is_saved_as_a_setting() {
    let mut s = Mount::desktop().start();
    // Added order: Starfall 64 is first and Dino Rush (PS2) is in the middle.
    assert!(top(&s, "Starfall 64") < top(&s, "Dino Rush"));

    s.click_label("Sort: Added");
    for option in ["Added", "Title", "System"] {
        assert!(s.has_label(option), "{option}: {:?}", s.labels());
    }
    s.take_effects();
    s.click_label("System");

    assert!(
        s.effects().contains(&Effect::Setting(SettingChange { app: None, key: "library_sort", value: SettingValue::Choice(2) })),
        "{:?}",
        s.effects()
    );
    assert_eq!(s.store().snapshot().settings.choice(SettingsTarget::Global, "library_sort", 0), 2);
    assert!(s.has_label("Sort: System"));
    // Nintendo first, Sony last; within the Nintendo 64 games, alphabetical.
    assert!(top(&s, "Kart Ruins") < top(&s, "Starfall 64"));
    assert!(top(&s, "Tide Racer") < top(&s, "Moon Garden") && top(&s, "Moon Garden") < top(&s, "Dino Rush"));
    s.snapshot("desktop-library-by-system");
}

#[test]
fn the_catalog_offers_system_chips_in_system_order() {
    let s = Mount::desktop().start_at(Route::Catalog {});
    let (n64, gba, ps2) = (top(&s, "Nintendo 64"), top(&s, "Game Boy Advance"), top(&s, "PlayStation 2"));
    let same_row = (n64 - gba).abs() < 1. && (gba - ps2).abs() < 1.;
    let x = |label: &str| s.label_box(label).expect("chip").0;
    assert!(same_row, "the chips share a row");
    assert!(
        x("Nintendo 64") < x("Game Boy Advance") && x("Game Boy Advance") < x("PlayStation 2"),
        "Nintendo's systems, oldest first, then Sony's"
    );
}

#[test]
fn deck_home_splits_into_a_shelf_per_system_when_sorted_by_system() {
    let mut s = Mount::deck().start();
    assert!(s.has_label("All apps") && !s.has_label("Nintendo 64"), "{:?}", s.labels());
    s.dispatch(AppAction::Setting(SettingChange { app: None, key: "library_sort", value: SettingValue::Choice(2) }));
    assert!(s.has_label("Nintendo 64"), "{:?}", s.labels());
    assert!(!s.has_label("All apps"));
    s.snapshot("deck-home-by-system");
}

#[test]
fn deck_tiles_carry_the_mark_of_their_system() {
    let mut s = Mount::deck().start();
    assert!(s.has_label("N64"), "{:?}", s.labels());
    s.snapshot("deck-home-badges");
}
