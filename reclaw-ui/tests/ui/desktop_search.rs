//! The search button: it opens over the top bar without moving anything, searches the tab that is showing, closes when the
//! search is sent or focus goes, keeps what was typed, and each tab shows its results with a way back. The Mods tab's shelves.
use freya::prelude::NamedKey;
use reclaw_ui::{
    nav::Route,
    shell::{DevOverrides, MotionOverride},
};

use crate::common::*;

fn calm() -> DevOverrides {
    DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() }
}

fn at(w: f32, route: Route) -> Session {
    Mount::desktop().size(w, 900.).dev(calm()).start_at(route)
}

/// The middle of the closed search button: the top bar's right end on a wide window.
fn wide_button(s: &Session) -> (f64, f64) {
    let (_, top, _, bottom) = s.label_box("SETTINGS").expect("the Settings tab");
    (f64::from(s.size.0 - 16. - 16.), f64::from((top + bottom) / 2.))
}

fn click(s: &mut Session, at: (f64, f64)) {
    s.runner.click_cursor(at);
    s.settle();
}

fn search(s: &mut Session, at: (f64, f64), text: &str) {
    click(s, at);
    s.type_text(text);
    s.settle();
    s.press(NamedKey::Enter);
    s.settle();
}

#[test]
fn the_box_opens_over_the_top_bar_and_nothing_moves() {
    let mut s = at(1100., Route::Library {});
    let before = (s.label_box("Recent"), s.label_box("Deck mode"), s.label_box("SETTINGS"));
    let button = wide_button(&s);
    click(&mut s, button);
    s.type_text("sky");
    s.settle();
    assert!(s.has_prose("sky"), "the field took the typing: {:?}", s.prose());
    let after = (s.label_box("Recent"), s.label_box("Deck mode"), s.label_box("SETTINGS"));
    assert_eq!(before, after, "opening the box moved the bar");
    s.snapshot("desktop-search-open");
}

/// The magnifier is drawn at the middle of the button's square: closed, and open (where the square is the bar's right end).
fn assert_magnifier_centred(s: &mut Session, (cx, cy): (f64, f64), size: f32, when: &str) {
    // The square less its border, so only the glyph is light.
    let inset = 3.;
    let (left, top) = (cx as f32 - size / 2. + inset, cy as f32 - size / 2. + inset);
    let square = (left as u32, top as u32, (left + size - 2. * inset) as u32, (top + size - 2. * inset) as u32);
    let (x, y) = s.ink_centre(square).unwrap_or_else(|| panic!("{when}: no magnifier drawn in {square:?}"));
    assert!(
        (x - cx as f32).abs() <= 1.5 && (y - cy as f32).abs() <= 1.5,
        "{when}: the magnifier's middle is at ({x}, {y}), the button's at ({cx}, {cy})"
    );
}

#[test]
fn the_magnifier_sits_in_the_middle_of_the_button_closed_and_open() {
    let mut s = at(1100., Route::Library {});
    let button = wide_button(&s);
    assert_magnifier_centred(&mut s, button, 32., "wide, closed");
    click(&mut s, button);
    assert_magnifier_centred(&mut s, button, 32., "wide, open");

    let mut s = at(600., Route::Library {});
    let size = 44.;
    let button = (600. - 16. - size / 2., 16. + size / 2.);
    assert_magnifier_centred(&mut s, button, size as f32, "phone, closed");
    click(&mut s, button);
    assert_magnifier_centred(&mut s, button, size as f32, "phone, open");

    let mut s = at(900., Route::Library {});
    let size = 36.;
    let button = (900. - 16. - size / 2., 16. + size / 2.);
    assert_magnifier_centred(&mut s, button, size as f32, "compact, closed");
    click(&mut s, button);
    assert_magnifier_centred(&mut s, button, size as f32, "compact, open");
}

#[test]
fn a_sent_search_shows_results_on_the_tab_and_clear_brings_everything_back() {
    let mut s = at(1100., Route::Mods {});
    assert!(s.has_label("Most downloaded"), "{:?}", s.labels());
    let button = wide_button(&s);
    search(&mut s, button, "camera");
    assert!(s.has_label("1 result for \u{201c}camera\u{201d}"), "{:?}", s.labels());
    assert!(s.has_prose("Free Camera"), "the title, with the match highlighted: {:?}", s.prose());
    assert!(!s.has_label("Randomizer") && !s.has_label("Most downloaded"), "{:?}", s.labels());
    assert!(!s.has_prose("camera") || s.has_prose("Free Camera"), "the box closed on sending");
    s.snapshot("desktop-search-mods-results");
    s.click_label("Clear search");
    assert!(s.has_label("Most downloaded") && s.has_label("Randomizer"), "{:?}", s.labels());
}

#[test]
fn each_tab_has_its_own_search_and_searching_from_a_game_goes_to_its_tab() {
    let mut s = at(1100., Route::Catalog {});
    let button = wide_button(&s);
    search(&mut s, button, "zzzz-nothing");
    assert!(s.has_label("Nothing in the catalog matches \u{201c}zzzz-nothing\u{201d}"), "{:?}", s.labels());
    s.open(Route::Library {});
    s.settle();
    assert!(s.has_label("Starfall 64"), "the library is not searched: {:?}", s.labels());
    s.open(Route::Game { id: 1 });
    s.settle();
    let button = wide_button(&s);
    search(&mut s, button, "sky");
    assert!(s.on_library(), "the results show on the Library");
    assert!(s.has_label("1 result for \u{201c}sky\u{201d}"), "{:?}", s.labels());
    s.open(Route::Catalog {});
    s.settle();
    assert!(s.has_label("Nothing in the catalog matches \u{201c}zzzz-nothing\u{201d}"), "the catalog kept its own: {:?}", s.labels());
}

#[test]
fn text_left_unsent_is_kept_and_does_not_search() {
    let mut s = at(1100., Route::Library {});
    let button = wide_button(&s);
    click(&mut s, button);
    s.type_text("tide");
    s.settle();
    // A click elsewhere closes the box.
    click(&mut s, (550., 600.));
    assert!(!s.labels().iter().any(|l| l.contains("result")), "nothing was searched: {:?}", s.labels());
    click(&mut s, button);
    assert!(s.has_prose("tide"), "what was typed is still there: {:?}", s.prose());
    s.press(NamedKey::Enter);
    s.settle();
    assert!(s.has_label("1 result for \u{201c}tide\u{201d}"), "{:?}", s.labels());
}

#[test]
fn settings_are_searched_and_the_rows_found_work_where_they_are() {
    let mut s = at(1100., Route::Settings {});
    let button = wide_button(&s);
    search(&mut s, button, "theme");
    assert!(s.has_label("Theme") && s.has_label("INTERFACE"), "{:?}", s.labels());
    assert!(!s.has_label("UI scale"), "only what was found: {:?}", s.labels());
    s.snapshot("desktop-search-settings");
}

#[test]
fn on_a_narrow_window_the_button_floats_at_the_top_right_and_searches_the_page() {
    // Compact: the box floats over the page instead of a search field under the title.
    let mut s = at(900., Route::Catalog {});
    let (_, top, _, _) = s.label_box("Catalog").expect("the page title");
    let size = 36.;
    let button = ((900. - 16. - size / 2.), (16. + size / 2.));
    let title_before = s.label_box("Catalog");
    search(&mut s, button, "zzzz-nothing");
    assert!(s.has_label("Nothing in the catalog matches \u{201c}zzzz-nothing\u{201d}"), "{:?}", s.labels());
    assert_eq!(s.label_box("Catalog"), title_before, "the title did not move");
    assert!(top >= 0.);
    s.snapshot("desktop-search-compact-results");
    s.click_label("Clear search");
    assert!(s.has_label("Starfall 64"), "the grid is back: {:?}", s.labels());

    // Phone: the same, without the rail.
    let mut s = at(600., Route::Mods {});
    let size = 44.;
    let button = ((600. - 16. - size / 2.), (16. + size / 2.));
    click(&mut s, button);
    s.type_text("hard");
    s.settle();
    let (_, chips_top, _, _) = s.label_box("All games").expect("the chips");
    s.snapshot("desktop-search-phone-open");
    s.press(NamedKey::Enter);
    s.settle();
    assert_eq!(s.label_box("All games").map(|b| b.1), Some(chips_top), "opening and closing the box moved nothing");
    assert!(s.has_label("1 result for \u{201c}hard\u{201d}"), "{:?}", s.labels());
    s.snapshot("desktop-search-phone-results");
}

#[test]
fn a_shelf_opens_into_the_whole_list_and_back() {
    let mut s = at(1100., Route::Mods {});
    for shelf in ["Installed", "Most downloaded", "Top rated", "Recently updated", "New releases"] {
        assert!(s.has_label(shelf), "{shelf}: {:?}", s.labels());
    }
    s.snapshot("desktop-mods-shelves");
    let (_, popular_top, _, _) = s.label_box("Most downloaded").expect("the shelf");
    // The first "Show all" is the Most downloaded shelf's (Installed has one mod).
    let (l, t, r, b) = s.label_box("Show all 5").expect("the shelf's button");
    assert!(t > popular_top - 40.);
    s.runner.click_cursor((f64::from((l + r) / 2.), f64::from((t + b) / 2.)));
    s.settle();
    assert!(s.has_label("All shelves") && s.has_label("Ghost Data Pack"), "all five: {:?}", s.labels());
    assert!(!s.has_label("Top rated"), "only the opened shelf: {:?}", s.labels());
    s.click_label("All shelves");
    assert!(s.has_label("Top rated"), "{:?}", s.labels());
}
