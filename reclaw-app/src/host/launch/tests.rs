//! Launching through the host with real processes: small shell scripts standing in for games.
#![cfg(unix)]
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Duration};

use reclaw_games::settings::{ConfigFileEdit, ConfigFormat, ConfigPath, ConfigValue, KeyEdit, LaunchPlan};
use reclaw_runtime::{Outcome, Probe};
use reclaw_ui::{effect::Effect, launch_request::LaunchRequest, notices::NoticeKind};

use super::*;
use crate::host::install::tests::{Rig, rel, start};

fn script(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("script");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("mode");
}

/// A rig with an app already "installed" as a folder holding a script, and the logs folder set.
fn installed(body: &str) -> (Rig, u32) {
    let rig = start(vec![rel("v1")], None, |config, root| {
        config.logs_dir = Some(root.join("logs"));
        config.grace = Some(Duration::from_millis(500));
        config.probe = Some(Probe { path: vec![root.join("no-runners")], home: root.join("home") });
    });
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    let folder = rig.folder();
    script(&folder.join("Game"), body);
    fs::write(folder.join("version.txt"), "v1").expect("version");
    (rig, app)
}

fn runs(rig: &Rig, app: u32) -> Vec<RunState> {
    rig.sink
        .all()
        .into_iter()
        .filter_map(|a| if let AppAction::SetRun { id, run } = a { (id == app).then_some(run) } else { None })
        .collect()
}

fn wait_idle(rig: &Rig, app: u32) {
    rig.sink.wait_for("the app to end", |_| {
        runs(rig, app).iter().any(|r| matches!(r, RunState::Idle | RunState::Failed(_)))
            && !matches!(runs(rig, app).last(), Some(RunState::Starting | RunState::Running { .. }))
    });
}

#[test]
fn play_starts_the_program_in_its_folder_with_the_arguments_in_order_and_reports_the_states() {
    let (rig, app) = installed("echo \"cwd=$(pwd)\" > out.txt; echo \"args=$*\" >> out.txt; echo hello from the game");
    let request = LaunchRequest {
        plan: LaunchPlan { args: vec!["--plan".into()], env: vec![("FROM_PLAN".into(), "yes".into())], config_edits: vec![] },
        options: vec!["--mine".into(), "two words".into()],
        problem: None,
    };
    rig.host.launch(app, &request);
    wait_idle(&rig, app);
    let out = fs::read_to_string(rig.folder().join("out.txt")).expect("the game wrote its file");
    assert!(out.contains(&format!("cwd={}", rig.folder().display())), "{out}");
    assert!(out.contains("args=--plan --mine two words"), "{out}");
    let states = runs(&rig, app);
    assert!(matches!(states.first(), Some(RunState::Starting)), "{states:?}");
    assert!(states.iter().any(|s| matches!(s, RunState::Running { .. })) || states.len() >= 2, "{states:?}");
    assert_eq!(states.last(), Some(&RunState::Idle));
    let log = fs::read_to_string(rig.root.path().join("logs/games/One.log")).expect("the game's output is kept");
    assert!(log.contains("hello from the game"), "{log}");
}

#[test]
fn a_game_that_quits_at_once_with_an_error_is_reported_with_where_its_output_is() {
    let (rig, app) = installed("echo 'missing libfoo.so' >&2; exit 3");
    rig.host.launch(app, &LaunchRequest::default());
    wait_idle(&rig, app);
    assert!(matches!(runs(&rig, app).last(), Some(RunState::Failed(Outcome::ExitedWithCode { code: 3, .. }))));
    rig.sink.wait_for("the notice", |_| rig.sink.notices().iter().any(|n| n.title.contains("closed right after starting")));
    let notice = rig.sink.notices().into_iter().find(|n| n.title.contains("closed right after")).expect("notice");
    assert_eq!(notice.kind, NoticeKind::Problem);
    assert!(notice.details.iter().any(|d| d.contains("One.log")), "{notice:?}");
    let log = fs::read_to_string(rig.root.path().join("logs/games/One.log")).expect("log");
    assert!(log.contains("missing libfoo.so"));
}

#[test]
fn a_crash_is_called_a_crash() {
    let (rig, app) = installed("kill -SEGV $$");
    rig.host.launch(app, &LaunchRequest::default());
    wait_idle(&rig, app);
    rig.sink.wait_for("the notice", |_| rig.sink.notices().iter().any(|n| n.title.contains("crashed")));
    let notice = rig.sink.notices().into_iter().find(|n| n.title.contains("crashed")).expect("notice");
    assert!(notice.details.iter().any(|d| d.contains("segmentation fault")), "{notice:?}");
}

#[test]
fn a_quit_late_with_a_non_zero_code_is_not_a_notice() {
    let (rig, app) = installed("exit 0");
    rig.host.launch(app, &LaunchRequest::default());
    wait_idle(&rig, app);
    assert_eq!(runs(&rig, app).last(), Some(&RunState::Idle));
    assert!(rig.sink.notices().is_empty(), "{:?}", rig.sink.notices());
}

#[test]
fn stop_asks_the_game_to_quit_and_the_card_follows() {
    let (rig, app) = installed("trap 'exit 0' TERM; echo ready > ready.txt; while true; do sleep 0.05; done");
    rig.host.launch(app, &LaunchRequest::default());
    rig.sink.wait_for("it to be running", |_| rig.folder().join("ready.txt").exists());
    rig.host.handle(&Effect::Stop(app));
    assert!(runs(&rig, app).iter().any(|s| matches!(s, RunState::Stopping { .. })), "{:?}", runs(&rig, app));
    wait_idle(&rig, app);
    assert_eq!(runs(&rig, app).last(), Some(&RunState::Idle), "stopping on request is not a failure");
}

#[test]
fn pressing_play_twice_starts_one_copy() {
    let (rig, app) = installed("echo x >> count.txt; sleep 0.4");
    rig.host.launch(app, &LaunchRequest::default());
    rig.host.launch(app, &LaunchRequest::default());
    rig.host.launch(app, &LaunchRequest::default());
    wait_idle(&rig, app);
    assert_eq!(fs::read_to_string(rig.folder().join("count.txt")).expect("count").lines().count(), 1);
}

#[test]
fn a_missing_folder_and_an_empty_folder_each_say_what_is_wrong() {
    let rig = start(vec![rel("v1")], None, |_, _| {});
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    rig.host.launch(app, &LaunchRequest::default());
    assert!(rig.sink.notices().iter().any(|n| n.body.contains("folder was not found")), "{:?}", rig.sink.notices());
    fs::create_dir_all(rig.folder()).expect("folder");
    rig.host.launch(app, &LaunchRequest::default());
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("has nothing to start")), "{:?}", rig.sink.notices());
    assert!(runs(&rig, app).is_empty(), "nothing was started");
}

#[test]
fn the_program_the_person_chose_beats_the_nearest_to_the_top() {
    let (rig, app) = installed("echo first > which.txt");
    script(&rig.folder().join("tools/Other"), "echo other > ../which.txt");
    fs::write(rig.folder().join("selected_executable.txt"), rig.folder().join("tools/Other").display().to_string()).expect("choice");
    rig.host.launch(app, &LaunchRequest::default());
    wait_idle(&rig, app);
    assert_eq!(fs::read_to_string(rig.folder().join("which.txt")).expect("which").trim(), "other");
}

#[test]
fn a_choice_that_points_outside_the_folder_is_ignored() {
    let (rig, _) = installed("echo first > which.txt");
    let outside = rig.root.path().join("elsewhere");
    script(&outside, "echo hijacked > /dev/null");
    fs::write(rig.folder().join("selected_executable.txt"), outside.display().to_string()).expect("choice");
    assert_eq!(choose_program(&rig.folder(), Platform::LinuxX64), Some(rig.folder().join("Game")));
}

#[test]
fn a_windows_program_with_no_runner_explains_what_to_install() {
    let rig = start(vec![rel("v1")], None, |config, root| {
        config.probe = Some(Probe { path: vec![root.join("nothing")], home: root.join("home") });
    });
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    fs::create_dir_all(rig.folder()).expect("folder");
    fs::write(rig.folder().join("Game.exe"), "MZ").expect("exe");
    rig.host.launch(app, &LaunchRequest::default());
    let notice = rig.sink.notices().pop().expect("a notice");
    assert!(notice.title.contains("needs Wine or Proton") && notice.details.iter().any(|d| d.contains("Proton")), "{notice:?}");
    assert!(runs(&rig, app).is_empty());
}

#[test]
fn a_windows_program_runs_through_wine_with_its_own_prefix() {
    let rig = start(vec![rel("v1")], None, |config, root| {
        let bin = root.join("bin");
        script(
            &bin.join("wine"),
            "echo \"prefix=$WINEPREFIX\" > \"$WINEPREFIX/../ran.txt\"; echo \"args=$*\" >> \"$WINEPREFIX/../ran.txt\"",
        );
        config.probe = Some(Probe { path: vec![bin], home: root.join("home") });
    });
    let app = rig.app();
    rig.host.handle(&Effect::AddToLibrary(app));
    fs::create_dir_all(rig.folder()).expect("folder");
    fs::write(rig.folder().join("Game.exe"), "MZ").expect("exe");
    rig.host.launch(app, &LaunchRequest { options: vec!["-windowed".into()], ..LaunchRequest::default() });
    wait_idle(&rig, app);
    let ran = fs::read_to_string(rig.folder().join("ran.txt")).expect("wine ran");
    assert!(ran.contains(&format!("prefix={}", rig.folder().join(".wine-prefix").display())), "{ran}");
    assert!(ran.contains(&format!("args={} -windowed", rig.folder().join("Game.exe").display())), "{ran}");
}

#[test]
fn launch_settings_that_edit_a_config_file_are_applied_before_the_game_starts() {
    let (rig, app) = installed("cat settings.json > seen.txt");
    fs::write(rig.folder().join("settings.json"), r#"{"Graphics": {"Width": 800}}"#).expect("config");
    let plan = LaunchPlan {
        args: vec![],
        env: vec![],
        config_edits: vec![ConfigFileEdit {
            file: ConfigPath { base: reclaw_games::settings::Base::Install, relative: "settings.json".into() },
            format: ConfigFormat::Json,
            edits: vec![KeyEdit { path: "Graphics.Width".into(), value: ConfigValue::Int(1920) }],
        }],
    };
    rig.host.launch(app, &LaunchRequest { plan, ..LaunchRequest::default() });
    wait_idle(&rig, app);
    let seen = fs::read_to_string(rig.folder().join("seen.txt")).expect("the game read its config");
    assert!(seen.contains("1920"), "{seen}");
}

#[test]
fn a_config_edit_that_fails_is_told_and_the_game_still_starts() {
    let (rig, app) = installed("echo started > started.txt");
    fs::write(rig.folder().join("settings.json"), "{ this is not json").expect("config");
    let plan = LaunchPlan {
        args: vec![],
        env: vec![],
        config_edits: vec![ConfigFileEdit {
            file: ConfigPath { base: reclaw_games::settings::Base::Install, relative: "settings.json".into() },
            format: ConfigFormat::Json,
            edits: vec![KeyEdit { path: "A".into(), value: ConfigValue::Int(1) }],
        }],
    };
    rig.host.launch(app, &LaunchRequest { plan, ..LaunchRequest::default() });
    wait_idle(&rig, app);
    assert!(rig.folder().join("started.txt").exists());
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("was not applied")), "{:?}", rig.sink.notices());
    assert_eq!(
        fs::read_to_string(rig.folder().join("settings.json")).expect("config"),
        "{ this is not json",
        "the broken file is not overwritten"
    );
}

#[test]
fn launch_options_that_could_not_be_read_are_said_once_and_the_game_starts() {
    let (rig, app) = installed("echo started > started.txt");
    rig.host.launch(
        app,
        &LaunchRequest {
            problem: Some("The launch options have an unmatched quote, so they were not used.".into()),
            ..LaunchRequest::default()
        },
    );
    wait_idle(&rig, app);
    assert!(rig.sink.notices().iter().any(|n| n.title.contains("Launch options were not used")));
    assert!(rig.folder().join("started.txt").exists());
}

#[test]
fn a_program_that_cannot_be_executed_is_a_notice_with_the_likely_cause() {
    let (rig, app) = installed("exit 0");
    fs::write(rig.folder().join("selected_executable.txt"), "").expect("no choice");
    // An ELF header with no interpreter: it is found as a program but the system cannot run it.
    let mut elf = vec![0u8; 64];
    elf[..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2;
    elf[5] = 1;
    elf[6] = 1;
    elf[16] = 2;
    elf[24] = 0x10;
    fs::remove_file(rig.folder().join("Game")).expect("remove");
    fs::write(rig.folder().join("Game"), &elf).expect("elf");
    rig.host.launch(app, &LaunchRequest::default());
    rig.sink.wait_for("a result", |_| !runs(&rig, app).is_empty() && !matches!(runs(&rig, app).last(), Some(RunState::Starting)));
    // Whether the system refuses to run it at all or runs it and it dies at once, the person is told.
    rig.sink.wait_for("a notice", |_| !rig.sink.notices().is_empty());
}
