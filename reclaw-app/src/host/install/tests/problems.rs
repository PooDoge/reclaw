use std::time::Duration;

use reclaw_ui::{activity::ActivityEvent, effect::Effect, model::AppStatus, notices::NoticeKind};

use super::*;

#[test]
fn a_location_that_is_not_a_full_path_is_refused_with_a_reason_and_nothing_starts() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: "Games".into(), prerelease: false });
    let notice = rig.sink.notices().pop().expect("a notice");
    assert_eq!(notice.kind, NoticeKind::Problem);
    assert!(notice.body.contains("not a full path") && notice.details.iter().any(|d| d.contains("/home/you/Games")), "{notice:?}");
    assert!(rig.activity().is_empty());
}

#[test]
fn a_release_with_nothing_for_this_system_fails_clearly_and_the_card_says_failed() {
    let rig = start(vec![Rel { tag: "v1", files: vec![("Game-macos.zip", vec![1])], slow: Duration::ZERO }], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_activity_ends();
    let events = rig.activity();
    let Some(ActivityEvent::Failed { reason, details, .. }) = events.last() else { panic!("{events:?}") };
    assert!(reason.contains("no recognized download for Linux-X64"), "{reason}");
    assert!(details.iter().any(|d| d.contains("reclaw.log")), "the log is named: {details:?}");
    assert_eq!(rig.status(), Some(AppStatus::Failed));

    // Fixing the release and trying again works, from Failed.
    *rig.releases.lock().expect("lock") = vec![rel("v2")];
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_ended(2);
    assert_eq!(rig.status(), Some(AppStatus::Installed));
}

#[test]
fn a_repository_with_no_releases_says_so() {
    let rig = start(vec![], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.wait_activity_ends();
    let events = rig.activity();
    assert!(matches!(events.last(), Some(ActivityEvent::Failed { reason, .. }) if reason.contains("o/one has no releases")), "{events:?}");
}

#[test]
fn cancelling_stops_the_download_and_puts_the_card_back() {
    let slow = Rel { tag: "v1", files: vec![("Game-linux.zip", game_zip("v1"))], slow: Duration::from_secs(5) };
    let rig = start(vec![slow], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    rig.sink.wait_for("the job to start", |all| all.iter().any(|a| matches!(a, AppAction::Activity(ActivityEvent::Started { .. }))));
    let id = rig.activity().iter().find_map(|e| if let ActivityEvent::Started { id, .. } = e { Some(*id) } else { None }).expect("started");
    rig.host.handle(&Effect::CancelActivity(id));
    rig.wait_activity_ends();
    assert!(matches!(rig.activity().last(), Some(ActivityEvent::Cancelled { .. })), "{:?}", rig.activity());
    assert_eq!(rig.status(), Some(AppStatus::Available));
    assert!(!rig.folder().join("version.txt").exists());
}

#[test]
fn a_second_press_while_one_is_running_does_nothing() {
    let slow = Rel { tag: "v1", files: vec![("Game-linux.zip", game_zip("v1"))], slow: Duration::from_millis(300) };
    let rig = start(vec![slow], None, |_, _| {});
    let app = rig.app();
    for _ in 0..3 {
        rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    }
    rig.wait_activity_ends();
    let started = rig.activity().iter().filter(|e| matches!(e, ActivityEvent::Started { .. })).count();
    assert_eq!(started, 1);
}

#[test]
fn uninstalling_refuses_a_folder_that_is_not_an_install() {
    let dir = tempfile::tempdir().expect("dir");
    let folder = dir.path().join("mine/One");
    std::fs::create_dir_all(&folder).expect("dirs");
    std::fs::write(folder.join("thesis.docx"), "years of work").expect("file");
    let library =
        format!(r#"{{"apps": [{{"name": "One", "repository": "o/one", "folderName": "One", "installPath": "{}"}}]}}"#, folder.display());
    let rig = start(vec![], Some(&library), |_, _| {});
    let app = rig.initial.loaded.games[0].id;
    rig.host.handle(&Effect::Uninstall(app));
    rig.sink.wait_for("the refusal", |_| {
        rig.sink.notices().iter().any(|n| n.kind == NoticeKind::Problem && n.title.contains("could not be uninstalled"))
    });
    assert!(folder.join("thesis.docx").exists());
}

#[test]
fn opening_the_folder_of_something_not_installed_says_where_it_looked() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    rig.host.handle(&Effect::OpenFolder(app));
    let notice = rig.sink.notices().pop().expect("a notice");
    assert_eq!(notice.kind, NoticeKind::Problem);
    assert!(notice.details.iter().any(|d| d.contains("default-apps")), "{notice:?}");
}

#[test]
fn a_catalog_entry_with_a_path_for_a_name_is_not_installed_anywhere() {
    let library = r#"{"apps": [{"name": "Bad", "repository": "o/bad", "folderName": "../../etc"}]}"#;
    let rig = start(vec![], Some(library), |_, _| {});
    let app = rig.initial.loaded.games[0].id;
    rig.host.handle(&Effect::StartInstall { app, location: String::new(), prerelease: false });
    let notice = rig.sink.notices().pop().expect("a notice");
    assert!(notice.body.contains("cannot be used"), "{notice:?}");
    assert!(rig.activity().is_empty());
}

#[test]
fn checking_the_update_of_something_not_installed_says_so() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    rig.host.handle(&Effect::CheckUpdate(app));
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("is not installed")));
}
