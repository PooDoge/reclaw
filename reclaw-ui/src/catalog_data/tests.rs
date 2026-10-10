use reclaw_catalog::{AppEntry, Extension, RepoSource, platform_index::PlatformEntry, timestamp::Timestamp};
use reclaw_sync::CatalogApp;

use super::*;

fn entry(name: &str, repo: &str, folder: &str, tags: &[&str]) -> AppEntry {
    AppEntry {
        name: name.into(),
        repository: repo.into(),
        folder_name: folder.into(),
        tags: tags.iter().map(|t| t.to_string()).collect(),
        ..Default::default()
    }
}

fn release(source: RepoSource, repo: &str, tag: &str) -> PlatformEntry {
    PlatformEntry {
        source,
        repository: repo.into(),
        preferred_release: None,
        release_tag: tag.into(),
        asset_names: vec![],
        validated_at: Timestamp::parse("2026-10-04T12:00:00Z").expect("a date"),
        selection_revision: 1,
    }
}

fn app(entry: AppEntry, tag: Option<&str>) -> CatalogApp {
    let release = tag.map(|t| release(entry.source, &entry.repository, t));
    CatalogApp { entry, lists: vec!["Nintendo".into()], release, site: None }
}

#[test]
fn a_number_is_the_same_on_every_start_and_in_every_order() {
    let keys = ["GITHUB:O/ONE:ONE", "GITHUB:O/TWO:TWO", "GITLAB:G/S/THREE:THREE"].map(String::from);
    let forward = IdMap::for_keys(keys.clone());
    let backward = IdMap::for_keys(keys.iter().rev().cloned());
    assert_eq!(forward, backward);
    assert_eq!(fnv1a("GITHUB:O/ONE:ONE"), fnv1a("GITHUB:O/ONE:ONE"));
    // A known value of 32-bit FNV-1a, so a change to the function (and every saved favorite) is noticed.
    assert_eq!(fnv1a("a"), 0xe40c_292c);
    assert_eq!(fnv1a(""), 0x811c_9dc5);
}

#[test]
fn two_keys_with_one_hash_get_different_numbers_whatever_the_order() {
    // Find a real collision: 32-bit hashes collide among a few hundred thousand strings.
    let mut seen = std::collections::HashMap::new();
    let pair = (0u32..2_000_000)
        .find_map(|i| {
            let key = format!("GITHUB:O/R{i}:R{i}");
            seen.insert(fnv1a(&key), key.clone()).map(|earlier| (earlier, key))
        })
        .expect("a collision exists");
    let (a, b) = pair;
    assert_eq!(fnv1a(&a), fnv1a(&b));
    let one = IdMap::for_keys([a.clone(), b.clone()]);
    let other = IdMap::for_keys([b.clone(), a.clone()]);
    assert_eq!(one, other);
    assert_ne!(one.get(&a), one.get(&b));
}

#[test]
fn keys_ignore_case_so_the_library_and_the_catalog_agree_on_a_game() {
    assert_eq!(key_of(&entry("A", "Owner/Repo", "Folder", &[])), key_of(&entry("A", "owner/repo", "folder", &[])));
}

#[test]
fn a_github_and_a_nested_gitlab_repository_are_split_and_linked() {
    let github = project_from(&app(entry("One", "o/one", "One", &[]), Some("v1.2.0")), 1);
    assert_eq!((github.repo.host, github.repo.owner.as_str(), github.repo.name.as_str()), (RepoHost::Github, "o", "one"));
    assert_eq!(github.releases[0].url, "https://github.com/o/one/releases/tag/v1.2.0");
    let gitlab_entry = AppEntry { source: RepoSource::Gitlab, ..entry("Three", "group/sub/three", "Three", &[]) };
    let gitlab = project_from(&app(gitlab_entry, Some("v2")), 3);
    assert_eq!((gitlab.repo.host, gitlab.repo.owner.as_str(), gitlab.repo.name.as_str()), (RepoHost::Gitlab, "group/sub", "three"));
    assert_eq!(gitlab.releases[0].url, "https://gitlab.com/group/sub/three/-/releases/v2");
    assert_eq!(gitlab.repo.url(), "https://gitlab.com/group/sub/three");
}

#[test]
fn what_the_catalog_gives_is_what_the_screens_get() {
    let mut e = entry("Super Mario 64", "harbourmasters/ghostship", "SuperMario64-Ghostship", &["recomp", "n64"]);
    e.project = Some("Ghostship".into());
    e.icon_url = Some("https://example.test/icon.png".into());
    let project = project_from(&app(e, Some("v8.0")), 5);
    assert_eq!((project.title.as_str(), project.project.as_str()), ("Super Mario 64", "Ghostship"));
    assert_eq!(project.platform, Platform::N64);
    assert_eq!(project.capsule_url.as_deref(), Some("https://example.test/icon.png"), "the icon is the best picture the catalog has");
    assert_eq!(project.hero_url, None, "and nothing is invented for the rest");
    assert!(project.summary.is_empty() && project.media.is_empty() && project.requirements.is_none());
    assert_eq!(project.tags, ["recomp", "n64"]);
    assert_eq!(project.latest_release().map(|r| r.tag.as_str()), Some("v8.0"));
}

#[test]
fn the_reclaw_block_adds_pictures_and_text_and_wins_over_the_icon() {
    let mut e = entry("Game", "o/g", "Game", &["n64"]);
    e.icon_url = Some("https://example.test/icon.png".into());
    e.extension = Some(Extension {
        summary: Some("A summary".into()),
        capsule_url: Some("https://example.test/capsule.png".into()),
        hero_url: Some("https://example.test/hero.jpg".into()),
        ..Default::default()
    });
    let project = project_from(&app(e.clone(), None), 1);
    assert_eq!(
        (project.summary.as_str(), project.capsule_url.as_deref(), project.hero_url.as_deref()),
        ("A summary", Some("https://example.test/capsule.png"), Some("https://example.test/hero.jpg"))
    );
    let game = game_from(&e, None, 1, None);
    assert_eq!(game.art.capsule.as_deref(), Some("https://example.test/capsule.png"));
}

#[test]
fn an_app_the_metadata_does_not_cover_has_no_release_and_an_empty_tag_is_no_release() {
    assert!(project_from(&app(entry("A", "o/a", "A", &[]), None), 1).releases.is_empty());
    assert!(project_from(&app(entry("A", "o/a", "A", &[]), Some("")), 1).releases.is_empty(), "checked, nothing usable");
}

#[test]
fn a_library_game_shows_the_users_name_and_the_catalogs_latest_version_and_is_not_installed() {
    let mut e = entry("Zelda", "o/z", "Zelda", &["n64"]);
    e.custom_display_name = Some("My Zelda".into());
    e.project = Some("Port".into());
    let game = game_from(&e, Some("v3"), 9, None);
    assert_eq!((game.title.as_ref(), game.project.as_ref(), game.version.as_ref()), ("My Zelda", "Port", "v3"));
    assert_eq!(
        (game.status, game.source, game.platform, game.run.clone()),
        (AppStatus::Available, Source::GitHub, Platform::N64, RunState::Idle)
    );
    assert_eq!(game_from(&entry("Mine", "", "Mine", &[]), None, 1, None).source, Source::Manual);
    assert_eq!(game_from(&AppEntry { source: RepoSource::Gitlab, ..entry("G", "g/p", "G", &[]) }, None, 1, None).source, Source::GitLab);
}

#[test]
fn loading_joins_the_library_to_the_catalog_by_identity_and_gives_both_the_same_number() {
    let catalog = vec![app(entry("One", "o/one", "One", &["n64"]), Some("v1")), app(entry("Two", "o/two", "Two", &["ps2"]), Some("v2"))];
    let library = vec![entry("Two", "O/TWO", "two", &["ps2", "mine"]), entry("Mine", "", "Mine", &[])];
    let loaded = load(&catalog, &library, &InstallStates::new());
    assert_eq!(loaded.projects.len(), 2);
    assert_eq!(loaded.games.len(), 2);
    let two = loaded.projects.iter().find(|p| p.title == "Two").expect("in the catalog");
    let game = loaded.games.iter().find(|g| g.title == "Two").expect("in the library");
    assert_eq!(game.id, two.id, "one game, one number");
    assert_eq!(game.version, "v2", "the library copy shows the catalog's latest version");
    assert!(loaded.games.iter().any(|g| g.title == "Mine" && g.source == Source::Manual));
    let ids: std::collections::HashSet<u32> = loaded.projects.iter().map(|p| p.id).chain(loaded.games.iter().map(|g| g.id)).collect();
    assert_eq!(ids.len(), 3, "three different things have three different numbers");
}

#[test]
fn an_empty_catalog_and_library_load_as_nothing_not_as_samples() {
    assert_eq!(load(&[], &[], &InstallStates::new()), Loaded::default());
}

#[test]
fn an_installed_game_shows_the_version_it_has_and_asks_for_an_update_only_when_the_catalog_has_a_newer_one() {
    let e = entry("Zelda", "o/z", "Zelda", &["n64"]);
    let installed = |v: &str| InstallState::Installed { version: v.to_string(), latest: None };

    let current = game_from(&e, Some("v1.4.2"), 1, Some(&installed("1.4.2")));
    assert_eq!((current.status, current.version.as_ref()), (AppStatus::Installed, "1.4.2"), "v1.4.2 and 1.4.2 are the same release");

    let behind = game_from(&e, Some("v1.5.0"), 1, Some(&installed("v1.4.2")));
    assert_eq!((behind.status, behind.version.as_ref()), (AppStatus::UpdateReady, "v1.4.2"), "it shows what is installed, not what is new");

    let ahead = game_from(&e, Some("v1.4.0"), 1, Some(&installed("v1.4.2")));
    assert_eq!(ahead.status, AppStatus::Installed, "the catalog being older is not an update");

    let unknown = game_from(&e, None, 1, Some(&installed("v1.4.2")));
    assert_eq!(unknown.status, AppStatus::Installed, "no news is not an update");

    let labelled = game_from(&e, Some("v1.4.2"), 1, Some(&installed("v1.4.2-beta")));
    assert_eq!(labelled.status, AppStatus::Installed, "a label does not make the same numbers newer");
}

#[test]
fn a_running_install_and_a_failed_one_show_as_such_with_the_version_that_was_wanted() {
    let e = entry("Zelda", "o/z", "Zelda", &["n64"]);
    let running = game_from(&e, Some("v2"), 1, Some(&InstallState::Installing));
    assert_eq!((running.status, running.version.as_ref()), (AppStatus::Installing, "v2"));
    let failed = game_from(&e, Some("v2"), 1, Some(&InstallState::Failed));
    assert_eq!(failed.status, AppStatus::Failed);
}

#[test]
fn loading_applies_each_games_state_by_its_key() {
    let library = vec![entry("One", "o/one", "One", &["n64"]), entry("Two", "o/two", "Two", &["ps2"])];
    let mut states = InstallStates::new();
    states.insert(key_of(&library[1]), InstallState::Installed { version: "v1".into(), latest: None });
    let loaded = load(&[], &library, &states);
    let status = |t: &str| loaded.games.iter().find(|g| g.title == t).map(|g| g.status);
    assert_eq!((status("One"), status("Two")), (Some(AppStatus::Available), Some(AppStatus::Installed)));
}

#[test]
fn a_newer_release_found_by_asking_the_host_service_counts_even_when_the_catalog_has_not_caught_up() {
    let e = entry("Zelda", "o/z", "Zelda", &["n64"]);
    let state = InstallState::Installed { version: "v1.0".into(), latest: Some("v1.1".into()) };
    assert_eq!(game_from(&e, Some("v1.0"), 1, Some(&state)).status, AppStatus::UpdateReady);
    let stale = InstallState::Installed { version: "v1.1".into(), latest: Some("v1.1".into()) };
    assert_eq!(game_from(&e, Some("v1.0"), 1, Some(&stale)).status, AppStatus::Installed, "nothing newer than what is installed");
}
