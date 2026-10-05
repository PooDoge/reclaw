use reclaw_install::Platform;
use reclaw_ui::{
    activity::{ActivityEvent, Kind, Stage},
    effect::Effect,
    model::AppStatus,
    notices::NoticeKind,
    settings::TextField,
};

use super::*;

#[test]
fn pressing_install_downloads_unpacks_records_and_reports_every_step() {
    let rig = start(vec![rel("v1.0.0")], None, |_, _| {});
    let app = rig.app();
    let location = rig.root.path().join("chosen");
    rig.host.handle(&Effect::StartInstall { app, location: location.display().to_string(), prerelease: false });
    rig.wait_activity_ends();

    let folder = location.join("One");
    assert!(folder.join("game.x86_64").is_file(), "the wrapper folder was hoisted");
    assert_eq!(std::fs::read_to_string(folder.join("version.txt")).expect("version"), "v1.0.0");
    assert_eq!(rig.status(), Some(AppStatus::Installed));
    let game = rig.sink.last_games().and_then(|g| g.into_iter().find(|g| g.title == "One")).expect("in the library");
    assert_eq!(game.version, "v1.0.0");

    // The library remembers where it went, in Quiver's field.
    let saved = std::fs::read_to_string(&rig.library_file).expect("library");
    assert!(saved.contains("installPath") && saved.contains(&location.join("One").display().to_string()), "{saved}");

    let events = rig.activity();
    assert!(matches!(&events[0], ActivityEvent::Started { kind: Kind::Install, title, .. } if title == "One"), "{events:?}");
    assert!(events.iter().any(|e| matches!(e, ActivityEvent::Progress { stage: Stage::Downloading, .. })));
    assert!(events.iter().any(|e| matches!(e, ActivityEvent::Progress { stage: Stage::Extracting, .. })));
    assert!(matches!(events.last(), Some(ActivityEvent::Finished { changelog: None, .. })), "{events:?}");
    // While it ran the card showed it.
    let during: Vec<_> = rig
        .sink
        .all()
        .into_iter()
        .filter_map(|a| if let AppAction::SetGames(g) = a { g.into_iter().find(|g| g.title == "One").map(|g| g.status) } else { None })
        .collect();
    assert!(during.contains(&AppStatus::Installing), "{during:?}");
}

#[test]
fn an_empty_location_means_the_default_from_settings() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: "  ".into(), prerelease: false });
    rig.wait_activity_ends();
    assert!(rig.folder().join("game.x86_64").is_file(), "{:?}", rig.sink.notices());
}

#[test]
fn changing_the_default_in_settings_moves_where_the_next_install_goes() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let elsewhere = rig.root.path().join("new-default");
    rig.host.handle(&Effect::TextCommitted { app: None, field: TextField::DefaultLocation, value: elsewhere.display().to_string() });
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_activity_ends();
    assert!(elsewhere.join("One/game.x86_64").is_file());
    assert!(!rig.folder().exists());
}

#[test]
fn a_tilde_in_the_location_is_the_home_folder() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: "~/Games".into(), prerelease: false });
    rig.wait_activity_ends();
    assert!(rig.root.path().join("home/Games/One/game.x86_64").is_file());
}

#[test]
fn what_is_already_installed_is_found_at_start_without_asking_anyone() {
    let dir = tempfile::tempdir().expect("dir");
    let folder = dir.path().join("somewhere/One");
    std::fs::create_dir_all(&folder).expect("dirs");
    std::fs::write(folder.join("version.txt"), "v0.9").expect("version");
    let library = format!(
        r#"{{"apps": [{{"name": "One", "repository": "o/one", "folderName": "One", "tags": ["n64"], "installPath": "{}"}}]}}"#,
        folder.display()
    );
    let rig = start(vec![], Some(&library), |_, _| {});
    let game = rig.initial.loaded.games.iter().find(|g| g.title == "One").expect("in the library");
    assert_eq!((game.status, game.version.as_ref()), (AppStatus::Installed, "v0.9"));
}

#[test]
fn an_unfinished_install_found_at_start_is_not_installed() {
    let dir = tempfile::tempdir().expect("dir");
    let folder = dir.path().join("One");
    std::fs::create_dir_all(&folder).expect("dirs");
    std::fs::write(folder.join("version.txt"), "v0.9").expect("version");
    std::fs::write(folder.join("install-incomplete.txt"), "v1").expect("marker");
    let library =
        format!(r#"{{"apps": [{{"name": "One", "repository": "o/one", "folderName": "One", "installPath": "{}"}}]}}"#, folder.display());
    let rig = start(vec![], Some(&library), |_, _| {});
    assert_eq!(rig.initial.loaded.games[0].status, AppStatus::Available);
}

#[test]
fn an_update_lays_the_new_release_over_the_old_and_reports_what_changed() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(1);
    std::fs::write(rig.folder().join("config.ini"), "mine").expect("config");

    *rig.releases.lock().expect("lock") = vec![rel("v2"), rel("v1")];
    rig.host.handle(&Effect::Update(app));
    rig.wait_ended(2);

    assert_eq!(std::fs::read_to_string(rig.folder().join("version.txt")).expect("version"), "v2");
    assert_eq!(std::fs::read_to_string(rig.folder().join("config.ini")).expect("config"), "mine");
    let events = rig.activity();
    let finished = events
        .iter()
        .rev()
        .find_map(|e| if let ActivityEvent::Finished { changelog, .. } = e { Some(changelog.clone()) } else { None })
        .expect("finished");
    let changelog = finished.expect("an update has a changelog");
    assert_eq!((changelog.from.as_str(), changelog.to.as_str(), changelog.notes.as_str()), ("v1", "v2", "Notes for v2"));
    assert!(events.iter().any(|e| matches!(e, ActivityEvent::Started { kind: Kind::Update, .. })));
}

#[test]
fn installing_what_is_current_says_so_and_downloads_nothing() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(1);
    let downloads = rig.server.requests().iter().filter(|r| r.path.starts_with("/dl/")).count();
    rig.host.handle(&Effect::Update(app));
    rig.wait_ended(2);
    assert_eq!(rig.server.requests().iter().filter(|r| r.path.starts_with("/dl/")).count(), downloads);
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("up to date")), "{:?}", rig.sink.notices());
    assert_eq!(rig.status(), Some(AppStatus::Installed));
}

#[test]
fn checking_for_updates_finds_a_newer_release_and_the_card_asks_for_the_update() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(1);
    *rig.releases.lock().expect("lock") = vec![rel("v1.1"), rel("v1")];
    rig.host.handle(&Effect::CheckUpdate(app));
    rig.sink.wait_for("the card to ask for the update", |all| {
        all.iter().any(|a| matches!(a, AppAction::SetGames(g) if g.iter().any(|g| g.status == AppStatus::UpdateReady)))
    });
    assert_eq!(rig.status(), Some(AppStatus::UpdateReady));

    *rig.releases.lock().expect("lock") = vec![rel("v1")];
    rig.host.handle(&Effect::CheckUpdate(app));
    rig.sink.wait_for("the all-clear", |_| rig.status() == Some(AppStatus::Installed));
    assert!(rig.sink.notices().iter().any(|n| n.kind == NoticeKind::Note && n.title.contains("up to date")));
}

#[test]
fn uninstalling_deletes_the_folder_and_the_app_stays_in_the_library() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(1);
    rig.host.handle(&Effect::Uninstall(app));
    rig.sink.wait_for("the uninstall", |_| rig.status() == Some(AppStatus::Available));
    assert!(!rig.folder().exists());
    let saved = std::fs::read_to_string(&rig.library_file).expect("library");
    assert!(saved.contains("\"One\"") && saved.contains("\"installPath\": null"), "{saved}");
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("was uninstalled")));
}

#[test]
fn verifying_reports_the_version_and_what_it_would_start() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(1);
    rig.host.handle(&Effect::Verify(app));
    let note = rig.sink.notices().into_iter().find(|n| n.title.contains("looks complete")).expect("a verdict");
    assert!(note.details.iter().any(|d| d.contains("v1")) && note.details.iter().any(|d| d.contains("game.x86_64")), "{note:?}");
    let _ = Platform::LinuxX64;
}
