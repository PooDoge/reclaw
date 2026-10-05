//! Pictures and READMEs from the internet, against a fake one: what is asked for and how often,
//! what shows while waiting and after, what happens when it fails or is switched off.
use reclaw_games::fixtures::sample_projects;
use reclaw_media::readme::ReadmeContext;
use reclaw_ui::{
    effect::Effect,
    model::Art,
    nav::Route,
    settings::{SettingChange, SettingValue},
    store::AppAction,
};

use crate::common::{web::*, *};

const HERO: &str = "https://art.example.com/hero/1.png";

/// The sample library with the first game (the one the wide Library shows a hero for) given a banner.
fn games_with_art() -> Vec<reclaw_ui::model::GameEntry> {
    let mut games = reclaw_ui::fixtures::sample_games();
    games[0].art = Art { capsule: None, hero: Some(HERO.to_string()) };
    games
}

/// Scroll the game page down to its last sections, where the README is.
fn scroll_to_readme(s: &mut Session) {
    s.wheel(600., 400., -4000.);
    s.pump(100);
    s.runner.sync_and_update();
}

fn readme_url() -> String {
    let repo = sample_projects().remove(0).repo;
    ReadmeContext::github(&repo.owner, &repo.name, "HEAD").expect("a valid repository").readme_url().to_string()
}

fn raw(path: &str) -> String {
    let repo = sample_projects().remove(0).repo;
    format!("https://raw.githubusercontent.com/{}/{}/HEAD/{path}", repo.owner, repo.name)
}

#[test]
fn a_banner_picture_replaces_its_placeholder_and_is_fetched_once() {
    let rig = Rig::new();
    rig.web.serve(HERO, png());
    let mut s = Mount::desktop().games(games_with_art()).media(rig.hub.clone()).start();
    s.pump(300);
    s.runner.sync_and_update();
    assert!(!s.has_label("HERO 16:5"), "the picture replaced it: {:?}", s.labels());
    assert_eq!(rig.web.asked_for(HERO), 1, "every redraw shares one request: {:?}", rig.web.asked());
    s.snapshot("desktop-library-art");
}

#[test]
fn without_a_hub_nothing_is_asked_for_and_the_placeholder_stays() {
    let mut s = Mount::desktop().games(games_with_art()).start();
    s.pump(300);
    assert!(s.has_label("HERO 16:5"), "{:?}", s.labels());
}

#[test]
fn a_picture_that_cannot_be_fetched_leaves_the_placeholder() {
    let rig = Rig::new(); // serves nothing: a 404
    let mut s = Mount::desktop().games(games_with_art()).media(rig.hub.clone()).start();
    s.pump(400);
    assert_eq!(rig.web.asked_for(HERO), 1);
    assert!(s.has_label("HERO 16:5"), "still a placeholder: {:?}", s.labels());
}

#[test]
fn switching_downloads_off_asks_for_nothing() {
    let rig = Rig::new();
    rig.web.serve(HERO, png());
    let mut s = Mount::desktop().media(rig.hub.clone()).start();
    s.dispatch(AppAction::Setting(SettingChange { app: None, key: "remote_media", value: SettingValue::Bool(false) }));
    s.dispatch(AppAction::SetGames(games_with_art()));
    s.pump(400);
    assert!(rig.web.asked().is_empty(), "{:?}", rig.web.asked());
    assert!(s.has_label("HERO 16:5"), "the placeholder stays: {:?}", s.labels());
}

#[test]
fn the_game_page_shows_the_readme_with_its_pictures_and_native_badges() {
    let rig = Rig::new();
    rig.web.serve(
        &readme_url(),
        "# Starfall 64 Recomp\n\nA **static** recompilation.\n\n[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://ci.example.com/run)\n\n![Menu](docs/menu.png)\n\n## Building from source\n\nRun `cargo build`.\n",
    );
    rig.web.serve(&raw("docs/menu.png"), png());
    let mut s = Mount::desktop().media(rig.hub.clone()).start_at(Route::Game { id: 1 });
    s.pump(700);
    s.runner.sync_and_update();
    scroll_to_readme(&mut s);

    assert!(s.has_label("README"), "{:?}", s.labels());
    assert!(s.has_prose("Building from source"), "the prose comes from the markdown viewer: {:?}", s.prose());
    assert!(s.has_prose("static"), "{:?}", s.prose());
    assert!(s.has_label("passing") && s.has_label("build"), "the badge is drawn natively: {:?}", s.labels());
    assert_eq!(rig.web.asked_for(&raw("docs/menu.png")), 1, "the screenshot, resolved against the repository");
    assert!(!rig.web.asked().iter().any(|u| u.contains("img.shields.io")), "a badge is never fetched: {:?}", rig.web.asked());
    s.snapshot("desktop-game-readme");
}

#[test]
fn pressing_a_badge_opens_its_link_through_the_host() {
    let rig = Rig::new();
    rig.web.serve(&readme_url(), "[![Build](https://img.shields.io/badge/build-passing-brightgreen)](https://ci.example.com/run)\n");
    let mut s = Mount::desktop().media(rig.hub.clone()).start_at(Route::Game { id: 1 });
    s.pump(600);
    s.runner.sync_and_update();
    scroll_to_readme(&mut s);
    s.take_effects();
    s.click_label("passing");
    assert_eq!(s.take_effects(), vec![Effect::OpenUrl("https://ci.example.com/run".to_string())]);
}

#[test]
fn a_long_readme_is_collapsed_until_asked_for() {
    let rig = Rig::new();
    let long: String = (1..=12).map(|i| format!("## Section {i}\n\n{} \n\n", "Words for the section. ".repeat(12))).collect();
    rig.web.serve(&readme_url(), long);
    let mut s = Mount::desktop().media(rig.hub.clone()).start_at(Route::Game { id: 1 });
    s.pump(600);
    s.runner.sync_and_update();
    scroll_to_readme(&mut s);
    assert!(s.has_label("Show the whole README"), "{:?}", s.labels());
    assert!(s.has_prose("Section 1") && !s.has_prose("Section 12"), "only the start: {:?}", s.prose());
    s.click_label("Show the whole README");
    assert!(s.has_prose("Section 12"));
    assert!(s.has_label("Show less"));
}

#[test]
fn a_readme_that_cannot_be_fetched_says_so_and_offers_the_repository() {
    let rig = Rig::new(); // a 404
    let mut s = Mount::desktop().media(rig.hub.clone()).start_at(Route::Game { id: 1 });
    s.pump(500);
    s.runner.sync_and_update();
    scroll_to_readme(&mut s);
    assert!(s.has_label("The README could not be loaded."), "{:?}", s.labels());
    s.take_effects();
    s.click_label("Open the repository");
    let repo = sample_projects().remove(0).repo;
    assert_eq!(s.take_effects(), vec![Effect::OpenUrl(repo.url())]);
}

#[test]
fn hostile_readme_html_shows_nothing_of_the_script() {
    let rig = Rig::new();
    rig.web
        .serve(&readme_url(), "<script>alert('pwned')</script>\n\n<p onclick=\"x()\">Visible words</p>\n\n[click](javascript:alert(1))\n");
    let mut s = Mount::desktop().media(rig.hub.clone()).start_at(Route::Game { id: 1 });
    s.pump(600);
    s.runner.sync_and_update();
    assert!(s.has_prose("Visible words"), "{:?}", s.prose());
    assert!(!s.prose().iter().any(|p| p.contains("pwned") || p.contains("javascript")), "{:?}", s.prose());
    assert!(s.has_prose("click"), "the words of the unsafe link stay: {:?}", s.prose());
}
