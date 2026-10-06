//! What the screens do with data that is not the fixtures': a catalog that arrives after the first frame, a list that goes empty and
//! full again, and, when `QUIVER_CATALOG_DIR` points at a checkout of the community catalog, the real thing.
use std::path::PathBuf;

use reclaw_catalog::{CatalogError, parse_list, platform_index::PlatformEntry};
use reclaw_sync::CatalogApp;
use reclaw_ui::{
    catalog_data::{self, Loaded},
    nav::Route,
    shell::{DevOverrides, MotionOverride},
    store::AppAction,
};

use freya::prelude::NamedKey;

use crate::common::*;

fn calm() -> DevOverrides {
    DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() }
}

fn apps_from(list_json: &str) -> Result<Vec<CatalogApp>, CatalogError> {
    Ok(parse_list(list_json)?
        .apps
        .into_iter()
        .map(|entry| CatalogApp { entry, lists: vec!["Test".into()], release: None::<PlatformEntry> })
        .collect())
}

const SMALL: &str = r#"{"name": "T", "version": "1", "apps": [
  {"name": "Alpha Quest", "project": "Alpha Port", "repository": "o/alpha", "folderName": "Alpha", "tags": ["n64"]},
  {"name": "Beta Racer", "repository": "o/beta", "folderName": "Beta", "tags": ["ps2", "playstation"]},
  {"name": "Gamma Strike", "repository": "o/gamma", "folderName": "Gamma", "tags": ["x360", "xbox"]}]}"#;

#[test]
fn a_catalog_that_arrives_after_the_first_frame_fills_the_page_without_breaking_it() {
    let mut s = Mount::desktop().size(1100., 900.).dev(calm()).games(vec![]).projects(vec![]).start_at(Route::Catalog {});
    assert!(s.has_label("Loading the catalog"), "empty first, and saying why: {:?}", s.labels());
    let Loaded { projects, games } = catalog_data::load(&apps_from(SMALL).expect("a catalog"), &[], &Default::default());
    s.dispatch(AppAction::SetProjects(projects));
    s.dispatch(AppAction::SetGames(games));
    s.settle();
    for title in ["Alpha Quest", "Beta Racer", "Gamma Strike"] {
        assert!(s.has_label(title), "{title}: {:?}", s.labels());
    }
    assert!(!s.has_label("Loading the catalog"));
    // The chips follow the systems the catalog really has: a PlayStation 2 game is not PlayStation, an Xbox 360 game is not an Xbox.
    for chip in ["Nintendo 64", "PlayStation 2", "Xbox 360"] {
        assert!(s.has_label(chip), "{chip}: {:?}", s.labels());
    }
    assert!(!s.has_label("PlayStation") && !s.has_label("Xbox"), "{:?}", s.labels());
}

#[test]
fn a_search_that_matches_nothing_and_is_then_cleared_does_not_break_the_grid() {
    let Loaded { projects, games } = catalog_data::load(&apps_from(SMALL).expect("a catalog"), &[], &Default::default());
    // In a compact window the search box sits under the page title.
    let mut s = Mount::desktop().size(900., 900.).dev(calm()).games(games).projects(projects).start_at(Route::Catalog {});
    assert!(s.has_label("Alpha Quest"));
    let (left, _, _, bottom) = s.label_box("3 recompilation projects").expect("the summary line");
    s.runner.click_cursor((f64::from(left + 120.), f64::from(bottom + 30.)));
    s.settle();
    s.type_text("zzzz-nothing");
    s.settle();
    assert!(s.has_label("No projects match"), "{:?}", s.labels());
    for _ in 0..12 {
        s.press(NamedKey::Backspace);
    }
    s.settle();
    assert!(s.has_label("Alpha Quest"), "the grid is back: {:?}", s.labels());
}

#[test]
fn the_first_start_is_explained_screen_by_screen_and_the_way_forward_works() {
    let mut library = Mount::desktop().size(1100., 900.).dev(calm()).games(vec![]).projects(vec![]).start_at(Route::Library {});
    for text in ["Your library is empty", "Browse the catalog"] {
        assert!(library.has_label(text), "{text}: {:?}", library.labels());
    }
    library.snapshot("first-start-library");
    // At the narrowest wide window the top bar's pieces sit side by side: the last tab ends before the first action begins.
    let (_, _, settings_right, _) = library.label_box("SETTINGS").expect("the Settings tab");
    let (recent_left, _, _, _) = library.label_box("Recent").expect("the Recent button");
    assert!(settings_right <= recent_left, "SETTINGS ends at {settings_right}, Recent starts at {recent_left}");
    library.click_label("Browse the catalog");
    library.settle();
    assert!(library.has_label("Loading the catalog"), "the catalog has not arrived: {:?}", library.labels());
    assert!(!library.has_label("No projects match"), "and it does not blame a search");
    library.snapshot("first-start-catalog");

    let mut mods = Mount::desktop().size(1100., 900.).dev(calm()).games(vec![]).projects(vec![]).start_at(Route::Mods {});
    mods.dispatch(AppAction::SetMods(vec![]));
    mods.settle();
    assert!(mods.has_label("No mods to show yet"), "{:?}", mods.labels());
}

// --- the real catalog, when there is a checkout of it ----------------------------------------------------------------------------

fn real_apps() -> Option<Vec<CatalogApp>> {
    let dir = PathBuf::from(std::env::var_os("QUIVER_CATALOG_DIR").or_else(|| {
        eprintln!("QUIVER_CATALOG_DIR is not set: skipping the real-catalog screens");
        None
    })?);
    let platform =
        reclaw_catalog::PlatformDocument::parse(&std::fs::read_to_string(dir.join("platform-index.json")).expect("platform-index.json"))
            .expect("valid");
    let index = reclaw_catalog::PlatformIndex::from_documents([&platform]);
    let mut apps: Vec<CatalogApp> = Vec::new();
    for name in ["Nintendo", "PlayStation", "Xbox", "OtherPlatforms"] {
        let text = std::fs::read_to_string(dir.join("community-app-catalog").join(format!("{name}.json"))).expect("a list");
        for entry in parse_list(&text).expect("a list").apps {
            let release = index.get(entry.source, &entry.repository, entry.preferred_version.as_deref()).cloned();
            apps.push(CatalogApp { entry, lists: vec![name.into()], release });
        }
    }
    Some(apps)
}

#[test]
fn the_real_catalog_fills_both_interfaces() {
    let Some(apps) = real_apps() else { return };
    let library: Vec<_> = apps.iter().step_by(40).map(|a| a.entry.for_library()).collect();
    let Loaded { projects, games } = catalog_data::load(&apps, &library, &Default::default());
    assert_eq!(projects.len(), apps.len());
    assert!(games.len() >= 5, "{}", games.len());

    let mut desktop =
        Mount::desktop().size(1400., 1000.).dev(calm()).games(games.clone()).projects(projects.clone()).start_at(Route::Catalog {});
    assert!(desktop.has_label(&format!("{} recompilation projects", projects.len())), "{:?}", desktop.labels());
    for chip in ["Nintendo 64", "PlayStation", "Xbox 360", "PC", "Mobile"] {
        assert!(desktop.has_label(chip), "{chip}: {:?}", desktop.labels());
    }
    desktop.snapshot("real-catalog-desktop-catalog");
    desktop.open_route(Route::Library {});
    desktop.settle();
    desktop.snapshot("real-catalog-desktop-library");

    // At a width where the last chip of a row has little room left, a count stays on its chip's line.
    let narrow = Mount::desktop().size(1280., 900.).dev(calm()).games(games.clone()).projects(projects.clone()).start_at(Route::Catalog {});
    let (_, label_top, _, _) = narrow.label_box("PlayStation").expect("the chip");
    let (_, count_top, _, count_bottom) = narrow
        .label_box(&projects.iter().filter(|p| p.platform == reclaw_games::project::Platform::Ps1).count().to_string())
        .expect("its count");
    assert!(
        (count_top - label_top).abs() < 6. && count_bottom - count_top < 24.,
        "the count sits beside its label, on one line: label at {label_top}, count {count_top}..{count_bottom}"
    );

    let mut deck = Mount::deck().size(1280., 800.).dev(calm()).games(games).projects(projects).start();
    deck.snapshot("real-catalog-deck-home");
}
