//! What the top of a game's page shows when the catalog names no banner picture: a wide picture from the
//! README, and failing that a colour taken from the game's name behind its icon. Each case is read from
//! the pixels the page draws, with flat-coloured fake pictures so the colour says which one it was.
use reclaw_games::fixtures::sample_projects;
use reclaw_media::readme::ReadmeContext;
use reclaw_ui::{
    model::GameEntry,
    nav::Route,
    settings::{SettingChange, SettingValue},
    store::AppAction,
};

use crate::common::{web::*, *};

const HERO: &str = "https://art.example.com/hero/1.png";
const ICON: &str = "https://art.example.com/icon/1.png";
const RED: (u8, u8, u8) = (200, 30, 30);
const GREEN: (u8, u8, u8) = (30, 190, 60);
const BLUE: (u8, u8, u8) = (30, 60, 220);

/// The sample library whose first game knows its repository (as a library entry from the catalog does) and has the given art.
fn games(hero: Option<&str>, icon: Option<&str>) -> Vec<GameEntry> {
    let mut games = reclaw_ui::fixtures::sample_games();
    games[0].art.repo = Some(sample_projects().remove(0).repo);
    games[0].art.hero = hero.map(str::to_string);
    games[0].art.capsule = icon.map(str::to_string);
    games
}

fn readme_url() -> String {
    let repo = sample_projects().remove(0).repo;
    ReadmeContext::github(&repo.owner, &repo.name, "HEAD").expect("a valid repository").readme_url().to_string()
}

fn raw(path: &str) -> String {
    let repo = sample_projects().remove(0).repo;
    format!("https://raw.githubusercontent.com/{}/{}/HEAD/{path}", repo.owner, repo.name)
}

fn open_game(rig: Option<&Rig>, games: Vec<GameEntry>) -> Session {
    let mount = Mount::desktop().games(games);
    let mount = match rig {
        Some(rig) => mount.media(rig.hub.clone()),
        None => mount,
    };
    let mut s = mount.start_at(Route::Game { id: 1 });
    s.pump(900);
    s.runner.sync_and_update();
    s
}

/// A pixel inside the banner, left of its middle (where the generated banner's icon sits) and above the title strip.
fn banner_pixel(s: &mut Session) -> (u8, u8, u8) {
    let (left, top, ..) = s.top_label_box("Starfall 64").expect("the banner's title");
    s.pixel((left + 300.) as u32, (top - 16. - 40.) as u32)
}

fn near(got: (u8, u8, u8), want: (u8, u8, u8)) -> bool {
    let d = |a: u8, b: u8| a.abs_diff(b);
    d(got.0, want.0) <= 6 && d(got.1, want.1) <= 6 && d(got.2, want.2) <= 6
}

#[test]
fn a_wide_readme_picture_becomes_the_banner_when_the_catalog_has_none() {
    let rig = Rig::new();
    rig.web.serve(&readme_url(), "![Banner](docs/banner.png)\n\nA game.\n");
    rig.web.serve(&raw("docs/banner.png"), solid_png(640, 200, RED));
    let mut s = open_game(Some(&rig), games(None, None));
    assert!(near(banner_pixel(&mut s), RED), "{:?}", banner_pixel(&mut s));
    assert_eq!(rig.web.asked_for(&raw("docs/banner.png")), 1, "the README and the banner share one request");
    s.snapshot("desktop-game-banner-from-readme");
}

#[test]
fn a_square_readme_picture_is_not_cropped_into_a_banner() {
    let rig = Rig::new();
    rig.web.serve(&readme_url(), "![Banner](docs/banner.png)\n");
    rig.web.serve(&raw("docs/banner.png"), solid_png(512, 512, GREEN));
    let mut with_square = open_game(Some(&rig), games(None, None));
    let shown = banner_pixel(&mut with_square);
    assert!(!near(shown, GREEN), "{shown:?}");

    // It is exactly what a game with no README at all gets.
    let mut bare = open_game(None, games(None, None));
    assert_eq!(shown, banner_pixel(&mut bare));
}

#[test]
fn the_catalogs_own_banner_wins_over_the_readme() {
    let rig = Rig::new();
    rig.web.serve(HERO, solid_png(1200, 375, BLUE));
    rig.web.serve(&readme_url(), "![Banner](docs/banner.png)\n");
    rig.web.serve(&raw("docs/banner.png"), solid_png(640, 200, RED));
    let mut s = open_game(Some(&rig), games(Some(HERO), None));
    assert!(near(banner_pixel(&mut s), BLUE), "{:?}", banner_pixel(&mut s));
}

#[test]
fn a_catalog_banner_that_is_gone_falls_through_to_the_readme() {
    let rig = Rig::new(); // the hero address is a 404
    rig.web.serve(&readme_url(), "![Banner](docs/banner.png)\n");
    rig.web.serve(&raw("docs/banner.png"), solid_png(640, 200, RED));
    let mut s = open_game(Some(&rig), games(Some(HERO), None));
    assert_eq!(rig.web.asked_for(HERO), 1);
    assert!(near(banner_pixel(&mut s), RED), "{:?}", banner_pixel(&mut s));
}

#[test]
fn the_generated_banner_is_a_colour_that_belongs_to_the_game() {
    let mut s = open_game(None, games(None, None));
    let first = banner_pixel(&mut s);
    let mut again = open_game(None, games(None, None));
    assert_eq!(first, banner_pixel(&mut again), "the same game is the same colour every time");

    let mut other = reclaw_ui::fixtures::sample_games();
    other[0].title = "Skyward Quest".into();
    let mut s_other = Mount::desktop().games(other).start_at(Route::Game { id: 1 });
    s_other.pump(600);
    let (left, top, ..) = s_other.top_label_box("Skyward Quest").expect("the banner's title");
    let other_pixel = s_other.pixel((left + 300.) as u32, (top - 16. - 40.) as u32);
    assert_ne!(first, other_pixel, "another game is another colour");
    s.snapshot("desktop-game-banner-generated");
}

#[test]
fn the_generated_banner_draws_the_icon_and_a_blurred_copy_behind_it() {
    let rig = Rig::new();
    rig.web.serve(ICON, solid_png(256, 256, GREEN));
    let mut s = open_game(Some(&rig), games(None, Some(ICON)));
    assert!(s.pictures() >= 2, "the backdrop and the icon: {}", s.pictures());
    assert_eq!(rig.web.asked_for(ICON), 1, "one request for both uses");
    s.snapshot("desktop-game-banner-generated-icon");
}

#[test]
fn with_downloads_switched_off_the_banner_is_still_a_colour_and_nothing_is_asked_for() {
    let rig = Rig::new();
    rig.web.serve(HERO, solid_png(1200, 375, BLUE));
    let mut s = Mount::desktop().media(rig.hub.clone()).start();
    s.dispatch(AppAction::Setting(SettingChange { app: None, key: "remote_media", value: SettingValue::Bool(false) }));
    s.dispatch(AppAction::SetGames(games(Some(HERO), None)));
    s.open(Route::Game { id: 1 });
    s.pump(600);
    assert!(rig.web.asked().is_empty(), "{:?}", rig.web.asked());
    assert!(!near(banner_pixel(&mut s), BLUE));
}
