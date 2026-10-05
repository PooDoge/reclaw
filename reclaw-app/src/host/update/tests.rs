use std::{
    fs,
    os::unix::fs::PermissionsExt,
    time::{Duration, Instant},
};

use reclaw_ui::{effect::Effect, notices::NoticeKind, store::AppAction};

use super::*;
use crate::host::{HostConfig, tests::Collector};

fn marker_line(result: &str, binary: &str) -> String {
    format!("RECLAW_UPDATE: {result} aaaaaaaaaa bbbbbbbbbb {binary}")
}

#[test]
fn the_scripts_last_line_is_read_and_anything_else_is_not_a_result() {
    assert_eq!(
        parse_marker("RECLAW_UPDATE: updated aaaaaaaaaa bbbbbbbbbb /x/target/release/reclaw"),
        Some(Marker {
            result: "updated".into(),
            old: "aaaaaaaaaa".into(),
            new: "bbbbbbbbbb".into(),
            binary: Some("/x/target/release/reclaw".into())
        })
    );
    assert_eq!(parse_marker("RECLAW_UPDATE: up-to-date aaaaaaaaaa aaaaaaaaaa none").map(|m| m.binary), Some(None));
    for not in ["==> building", "RECLAW_UPDATE: updated", "RECLAW_UPDATE:", "", "updated a b c"] {
        assert_eq!(parse_marker(not), None, "{not:?}");
    }
}

fn finished(code: Option<i32>, result: Option<&str>, tail: &[&str]) -> Finished {
    Finished {
        code,
        marker: result.map(|r| parse_marker(&marker_line(r, "/x/reclaw")).expect("marker")),
        spawn_error: None,
        commits: vec!["abc1234 first change".into(), "def5678 second change".into()],
        tail: tail.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn every_outcome_of_the_script_has_a_notice_that_says_what_to_do() {
    let updated = notice_for(&finished(Some(0), Some("updated"), &[]));
    assert_eq!((updated.title.as_str(), updated.kind), ("Reclaw updated", NoticeKind::Note));
    assert!(updated.body.contains("aaaaaaaaaa to bbbbbbbbbb") && updated.body.contains("Quit and start Reclaw again"), "{}", updated.body);
    assert_eq!(updated.details, ["abc1234 first change", "def5678 second change", "Built: /x/reclaw"]);

    assert_eq!(notice_for(&finished(Some(0), Some("up-to-date"), &[])).title, "Reclaw is up to date");
    assert_eq!(notice_for(&finished(Some(0), Some("ahead"), &[])).title, "Nothing to update");
    assert_eq!(notice_for(&finished(Some(0), None, &["odd"])).title, "The update finished");

    let cases = [
        (4, "The update stopped: you have local changes", "Nothing was changed"),
        (3, "The update stopped: the histories differ", "Nothing was changed"),
        (5, "The update could not reach the remote", "Check the network connection"),
        (6, "The build failed", "The new source is in place; fix the error and update again"),
        (2, "The update cannot run here", "This checkout cannot be updated"),
    ];
    for (code, title, body) in cases {
        let notice = notice_for(&finished(Some(code), None, &["the reason, as the script said it"]));
        assert_eq!((notice.title.as_str(), notice.body.as_str(), notice.kind), (title, body, NoticeKind::Problem), "exit {code}");
        assert!(
            notice.details.iter().any(|d| d.contains("the reason")),
            "the script's own words are passed on (exit {code}): {:?}",
            notice.details
        );
    }
    let odd = notice_for(&finished(Some(77), None, &[]));
    assert_eq!((odd.title.as_str(), odd.body.as_str()), ("The update failed", "The script exited with 77"));
    assert_eq!(notice_for(&finished(None, None, &[])).title, "The update was interrupted");
    let no_bash = notice_for(&Finished { spawn_error: Some("bash could not be started: not found".into()), ..Finished::default() });
    assert_eq!((no_bash.title.as_str(), no_bash.kind), ("The update could not start", NoticeKind::Problem));
}

#[test]
fn a_failure_shows_the_end_of_the_output_not_the_beginning() {
    let lines: Vec<String> = (0..60).map(|n| format!("line {n}")).collect();
    let refs: Vec<&str> = lines.iter().map(String::as_str).collect();
    let notice = notice_for(&finished(Some(6), None, &refs));
    assert_eq!(notice.details.len(), 15);
    assert_eq!(notice.details.last().map(String::as_str), Some("line 59"));
    assert_eq!(notice.details.first().map(String::as_str), Some("line 45"));
}

struct Rig {
    host: Host,
    sink: Arc<Collector>,
    dir: tempfile::TempDir,
}

/// A host whose "checkout" has a stand-in `scripts/update.sh` with the given body.
fn rig(script: Option<&str>) -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let scripts = dir.path().join("checkout").join("scripts");
    fs::create_dir_all(&scripts).expect("mkdir");
    fs::create_dir_all(dir.path().join("checkout").join(".git")).expect("fake .git");
    let update = script.map(|body| {
        let path = scripts.join("update.sh");
        fs::write(&path, format!("#!/usr/bin/env bash\n{body}\n")).expect("script");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).expect("chmod");
        UpdateSource::detect(dir.path().join("checkout").to_str().expect("utf8"), "debug").expect("detected")
    });
    let sink = Arc::new(Collector::default());
    let (host, _) = Host::open(HostConfig { update, ..HostConfig::new(None, dir.path().join("apps.json")) }, sink.clone());
    Rig { host, sink, dir }
}

impl Rig {
    fn wait_for_notice(&self, title: &str) -> reclaw_ui::notices::Notice {
        let end = Instant::now() + Duration::from_secs(20);
        while Instant::now() < end {
            if let Some(n) = self.sink.notices().into_iter().find(|n| n.title == title) {
                return n;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no notice {title:?}; got {:?}", self.sink.notices().iter().map(|n| n.title.clone()).collect::<Vec<_>>());
    }
}

#[test]
fn a_successful_update_runs_the_script_with_this_builds_profile_and_reports_it() {
    let rig = rig(Some(
        "echo \"$@\" > \"$(dirname \"$0\")/../args.txt\"\n\
         echo \"==> new commits:\"\n\
         echo \"    abc1234 first change\"\n\
         echo \"RECLAW_UPDATE: updated aaaaaaaaaa bbbbbbbbbb /x/reclaw\"",
    ));
    rig.host.handle(&Effect::UpdateSources);
    assert_eq!(rig.sink.notices()[0].title, "Updating Reclaw", "the person sees it started");
    let done = rig.wait_for_notice("Reclaw updated");
    assert_eq!(done.details[0], "abc1234 first change");
    let args = fs::read_to_string(rig.dir.path().join("checkout").join("args.txt")).expect("args");
    assert_eq!(args.trim(), "--profile debug", "the rebuild matches the profile that is running");
}

#[test]
fn a_failing_script_gives_its_message_and_a_second_update_can_follow() {
    let rig =
        rig(Some("echo 'error: could not compile reclaw-ui' >&2\necho \"RECLAW_UPDATE: build-failed aaaaaaaaaa bbbbbbbbbb none\"\nexit 6"));
    rig.host.handle(&Effect::UpdateSources);
    let notice = rig.wait_for_notice("The build failed");
    assert!(notice.details.iter().any(|d| d.contains("could not compile reclaw-ui")), "{:?}", notice.details);
    // The guard is released: a later attempt starts again.
    let end = Instant::now() + Duration::from_secs(5);
    while rig.host.inner.updating.load(Ordering::SeqCst) && Instant::now() < end {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!rig.host.inner.updating.load(Ordering::SeqCst));
}

#[test]
fn a_second_request_while_one_runs_is_told_to_wait_and_does_not_start_another() {
    let rig = rig(Some("sleep 1\necho \"RECLAW_UPDATE: up-to-date aaaaaaaaaa aaaaaaaaaa none\""));
    rig.host.handle(&Effect::UpdateSources);
    rig.host.handle(&Effect::UpdateSources);
    let titles: Vec<String> = rig.sink.notices().into_iter().map(|n| n.title).collect();
    assert_eq!(titles, ["Updating Reclaw", "An update is already running"]);
    rig.wait_for_notice("Reclaw is up to date");
}

#[test]
fn a_copy_not_built_from_a_checkout_says_so_and_how_to_get_one() {
    let rig = rig(None);
    rig.host.handle(&Effect::UpdateSources);
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("This copy cannot update itself", NoticeKind::Note));
    assert!(notice.details.iter().any(|d| d.contains("git clone")), "{:?}", notice.details);
    assert!(rig.sink.all().iter().all(|a| matches!(a, AppAction::Notify(_) | AppAction::Credentials { .. })));
}

#[test]
fn a_checkout_that_is_gone_or_has_no_script_is_not_a_source() {
    assert!(UpdateSource::detect("", "debug").is_none());
    assert!(UpdateSource::detect("/nonexistent/checkout", "release").is_none());
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join(".git")).expect("git");
    assert!(UpdateSource::detect(dir.path().to_str().expect("utf8"), "release").is_none(), "no scripts/update.sh");
    assert_eq!(UpdateSource::detect(dir.path().to_str().expect("utf8"), "weird").map(|s| s.profile), None);
}
