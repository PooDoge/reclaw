//! The routed desktop pages, drawn from the sample library and catalog.
use reclaw_ui::{
    effect::Effect,
    model::ModProvider,
    nav::Route,
    shell::{DevOverrides, MotionOverride},
};

use crate::common::*;

fn calm() -> DevOverrides {
    DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() }
}

fn at(route: Route) -> Session {
    Mount::desktop().size(1100., 900.).dev(calm()).start_at(route)
}

#[test]
fn the_game_page_shows_everything_a_project_publishes() {
    let mut s = at(Route::Game { id: 1 });
    for text in [
        "Starfall 64",
        "Play",
        "Favorite",
        "Settings",
        "ABOUT",
        "SCREENSHOTS AND VIDEOS",
        "Hub world at 1440p",
        "Launch trailer",
        "RECENT UPDATES",
        "v1.4.2",
        "Controller fixes",
        "SYSTEM REQUIREMENTS",
        "Minimum",
        "Recommended",
        "MODS FOR THIS GAME",
        "HD Texture Pack",
        "LINKS",
        "Project page",
    ] {
        assert!(s.has_label(text), "{text}: {:?}", s.labels());
    }
    s.snapshot("desktop-game-wide");
}

#[test]
fn a_project_with_no_extras_hides_the_empty_sections() {
    let s = at(Route::Game { id: 4 });
    assert!(s.has_label("Install"), "{:?}", s.labels());
    for text in ["SCREENSHOTS AND VIDEOS", "RECENT UPDATES", "SYSTEM REQUIREMENTS", "MODS FOR THIS GAME"] {
        assert!(!s.has_label(text), "{text} has nothing to show for Dino Rush");
    }
}

#[test]
fn the_star_marks_a_favorite_and_the_cog_opens_the_games_settings() {
    let mut s = at(Route::Game { id: 1 });
    s.click_label("Favorite");
    assert_eq!(s.take_effects(), vec![Effect::ToggleFavorite(1)]);
    assert!(s.has_label("Favorited"), "the button follows the store: {:?}", s.labels());
    assert!(s.store().snapshot().favorites.contains(&1));

    s.click_label("Settings");
    assert!(s.has_label("Properties".to_uppercase().as_str()) || s.has_label("PROPERTIES"), "{:?}", s.labels());
    assert!(s.has_label("Display and graphics"), "{:?}", s.labels());
}

#[test]
fn a_video_opens_in_the_system_player_through_the_host() {
    let mut s = at(Route::Game { id: 1 });
    // The trailer is the fourth tile; the strip scrolls sideways.
    s.runner.scroll((300., 690.), (-500., 0.));
    s.settle();
    s.click_label("Launch trailer");
    assert_eq!(s.take_effects(), vec![Effect::OpenUrl("https://example.com/starfall64-trailer".into())]);
}

#[test]
fn the_catalog_lists_every_project_and_filters_by_system() {
    let mut s = at(Route::Catalog {});
    assert!(s.has_label("Catalog") && s.has_label("Dino Rush") && s.has_label("Moon Garden"), "{:?}", s.labels());
    s.snapshot("desktop-catalog");
    s.click_label("PlayStation 2");
    assert!(s.has_label("Dino Rush") && !s.has_label("Starfall 64"), "{:?}", s.labels());
}

#[test]
fn mods_list_filter_install_and_open() {
    let mut s = at(Route::Mods {});
    assert!(s.has_label("HD Texture Pack") && s.has_label("Randomizer"), "{:?}", s.labels());
    s.snapshot("desktop-mods");
    s.click_label("GameBanana");
    assert!(s.has_label("Randomizer") && !s.has_label("HD Texture Pack"), "{:?}", s.labels());
    s.click_label("Install");
    assert!(matches!(s.take_effects().first(), Some(Effect::InstallMod { provider: ModProvider::GameBanana, .. })));
    assert!(s.has_label("Installing"), "the row shows it at once: {:?}", s.labels());
}

#[test]
fn a_mod_page_shows_its_details_and_links_to_its_game() {
    let mut s = at(Route::ModDetail { provider: "thunderstore".into(), mod_id: "hd-textures".into() });
    for text in ["HD Texture Pack", "pixelwright", "2.1.0", "For Starfall 64"] {
        assert!(s.has_label(text), "{text}: {:?}", s.labels());
    }
    s.snapshot("desktop-mod-detail");
    s.click_label("For Starfall 64");
    assert!(s.has_label("Play"), "the game's page: {:?}", s.labels());
}

#[test]
fn a_page_for_something_unknown_says_so_and_offers_the_way_out() {
    let mut s = at(Route::Game { id: 999 });
    assert!(s.has_label("No such game"), "{:?}", s.labels());
    s.click_label("Go to Library");
    assert!(s.has_label("Favorites"), "{:?}", s.labels());
    let s2 = at(Route::ModDetail { provider: "nope".into(), mod_id: "x".into() });
    assert!(s2.has_label("No such mod"), "{:?}", s2.labels());
}

#[test]
fn downloads_show_progress_failure_and_a_way_to_clear_a_row() {
    let mut s = at(Route::Downloads {});
    for text in ["Skyward Quest v0.9.2", "Downloading", "21 MB of 61 MB", "Moon Garden v2.0.1", "Failed", "Ghost Data Pack"] {
        assert!(s.has_label(text), "{text}: {:?}", s.labels());
    }
    s.snapshot("desktop-downloads");
}

#[test]
fn the_library_sidebar_has_the_updates_section() {
    let mut s = Mount::desktop().dev(calm()).start();
    assert!(s.has_label("UPDATES (3)"), "{:?}", s.labels());
    assert!(s.has_label("34% · 6 s left · 7.4 MB/s") && s.has_label("1 mod downloading"), "{:?}", s.labels());
    s.snapshot("desktop-library-updates");
}

#[test]
fn a_game_on_quiverlauncher_com_shows_how_it_runs_its_reviews_its_releases_and_who_made_it() {
    use std::collections::BTreeMap;

    use reclaw_catalog::site::{Detail, HistoryRelease, ReleaseState, Review, RunResult, SiteApp, SiteProject, Verified};
    use reclaw_ui::{
        community::{PageData, PageState},
        store::AppAction,
    };

    let mut s = at(Route::Game { id: 1 });
    assert!(!s.has_label("HOW IT RUNS"), "a game the site does not list has none of it");
    assert!(s.take_effects().is_empty(), "nothing to read for a game with no entry");

    let app = SiteApp {
        slug: "starfall-64".into(),
        recommended: 2,
        report_issues: 1,
        supported_os: vec!["linux".into()],
        verified: Some(Verified { version: "v1.4.1".into(), ..Default::default() }),
        ..Default::default()
    };
    s.dispatch(AppAction::SetCommunity(BTreeMap::from([(1, app.clone())])));
    assert!(s.take_effects().contains(&Effect::LoadCommunity(1)), "linked after the page opened: its page is read now");
    assert!(s.has_label("Runs well") && s.has_label("Loading reviews…"), "{:?}", s.labels());

    let page = PageData {
        detail: Ok(Some(Detail {
            entry: app,
            project: SiteProject { author: Some("Starfall Team".into()), ..Default::default() },
            ..Default::default()
        })),
        reviews: Ok(vec![Review { author: "Sam".into(), result: RunResult::Issues, body: "Audio crackles.".into(), ..Default::default() }]),
        releases: Ok(vec![
            HistoryRelease {
                version: "v1.5.0".into(),
                state: ReleaseState::Blocked,
                reasons: vec!["A file was replaced.".into()],
                ..Default::default()
            },
            HistoryRelease { version: "v1.4.1".into(), state: ReleaseState::Verified, ..Default::default() },
        ]),
    };
    s.dispatch(AppAction::CommunityPage { id: 1, page: PageState::Loaded(Box::new(page)) });
    for text in [
        "HOW IT RUNS",
        " · 2 run well, 1 with issues",
        "Sam",
        "Runs with issues",
        "Audio crackles.",
        "RELEASES",
        "BLOCKED",
        "A file was replaced.",
        "ON QUIVERLAUNCHER.COM",
        "Starfall Team",
        "Linux",
    ] {
        assert!(s.has_label(text), "{text}: {:?}", s.labels());
    }
    assert!(!s.has_label("RECENT UPDATES"), "the site's releases take the catalog's place");
    s.snapshot("desktop-game-community");
    s.click_label("Share how it runs");
    assert_eq!(s.take_effects(), vec![Effect::OpenUrl("https://quiverlauncher.com/apps/starfall-64?tab=how-it-runs".into())]);
}
