use super::{a_mod, progress, started};
use crate::{activity::*, model::AppStatus, sample::sample_games};

fn game(id: u32) -> crate::model::GameEntry {
    sample_games().into_iter().find(|g| g.id == id).expect("sample game")
}

fn board(events: Vec<ActivityEvent>) -> ActivityBoard {
    let mut b = ActivityBoard::new();
    for e in events {
        b.apply(e);
    }
    b
}

#[test]
fn an_idle_installed_game_shows_nothing_and_an_update_waiting_shows_update_ready() {
    let b = ActivityBoard::new();
    assert_eq!(indicator_for(&b, &game(1)), None);
    let i = indicator_for(&b, &game(2)).expect("Skyward Quest has an update ready");
    assert_eq!((i.kind, i.label.as_str(), i.is_active()), (IndicatorKind::UpdateAvailable, "Update ready", false));
}

#[test]
fn a_download_shows_percent_and_a_build_shows_the_step() {
    let b = board(vec![started(1, 2, Kind::Update, Some(1000)), progress(1, Stage::Downloading, 340, Some(1000), Some(10))]);
    let i = indicator_for(&b, &game(2)).expect("indicator");
    assert_eq!((i.kind, i.label.as_str(), i.progress), (IndicatorKind::Downloading, "34%", Some(0.34)));
    assert!(i.is_active());
    let b = board(vec![started(1, 2, Kind::Update, Some(1000)), progress(1, Stage::Building, 1000, Some(1000), None)]);
    let i = indicator_for(&b, &game(2)).expect("indicator");
    assert_eq!((i.kind, i.label.as_str()), (IndicatorKind::Installing, "Building"));
}

#[test]
fn running_work_beats_a_waiting_update_and_a_finished_update_replaces_it() {
    let b = board(vec![started(1, 2, Kind::Update, None), ActivityEvent::Finished { id: 1, changelog: None }]);
    let i = indicator_for(&b, &game(2)).expect("indicator");
    assert_eq!(
        (i.kind, i.label.as_str()),
        (IndicatorKind::Done, "Updated"),
        "done is shown even though the library still says update ready"
    );
}

#[test]
fn mods_downloading_are_counted_and_alone_they_still_get_an_indicator() {
    let b = board(vec![
        started(1, 1, a_mod(), Some(100)),
        started(2, 1, a_mod(), Some(100)),
        progress(1, Stage::Downloading, 50, Some(100), None),
    ]);
    let i = indicator_for(&b, &game(1)).expect("indicator");
    assert_eq!((i.kind, i.label.as_str(), i.mods), (IndicatorKind::Mods, "2 mods", 2));
    assert_eq!(i.progress, Some(0.25), "the average of 50% and 0%");
}

#[test]
fn a_failure_is_shown_until_something_newer_runs() {
    let b = board(vec![started(1, 5, Kind::Update, None), ActivityEvent::Failed { id: 1, reason: "asset missing".into() }]);
    let i = indicator_for(&b, &game(5)).expect("indicator");
    assert_eq!(i.kind, IndicatorKind::Failed);
    let b2 = board(vec![
        started(1, 5, Kind::Update, None),
        ActivityEvent::Failed { id: 1, reason: "x".into() },
        started(2, 5, Kind::Update, None),
    ]);
    assert_eq!(indicator_for(&b2, &game(5)).map(|i| i.kind), Some(IndicatorKind::Queued));
}

#[test]
fn the_sidebar_lists_active_then_failed_then_waiting_then_done() {
    let games = sample_games();
    let b = board(vec![
        // Starfall 64: a finished update with release notes.
        started(1, 1, Kind::Update, Some(10)),
        ActivityEvent::Finished {
            id: 1,
            changelog: Some(Changelog { from: "v1.4.0".into(), to: "v1.4.2".into(), notes: "Fixes".into(), url: None }),
        },
        // Kart Ruins: downloading.
        started(2, 3, Kind::Install, Some(1000)),
        progress(2, Stage::Downloading, 500, Some(1000), Some(100)),
        // Moon Garden: failed.
        started(3, 5, Kind::Update, None),
        ActivityEvent::Failed { id: 3, reason: "Release asset not found".into() },
    ]);
    let entries = sidebar_entries(&b, &games);
    let order: Vec<(u32, SidebarKind)> = entries.iter().map(|e| (e.game_id, e.kind)).collect();
    // Skyward Quest (2) still has "update ready" and nothing started.
    assert_eq!(order, vec![(3, SidebarKind::Active), (5, SidebarKind::Failed), (2, SidebarKind::Available), (1, SidebarKind::Done)]);
    let by = |id| entries.iter().find(|e| e.game_id == id).expect("entry");
    assert_eq!(by(3).detail, "50% · 5 s left · 100 B/s");
    assert_eq!(by(5).detail, "Release asset not found");
    assert_eq!(by(2).detail, "Update ready");
    assert_eq!(by(1).detail, "Updated to v1.4.2");
}

#[test]
fn a_game_with_an_update_and_mods_in_flight_is_one_entry() {
    let games = sample_games();
    let b = board(vec![
        started(1, 2, Kind::Update, Some(100)),
        progress(1, Stage::Downloading, 10, Some(100), None),
        started(2, 2, a_mod(), Some(100)),
    ]);
    let entries = sidebar_entries(&b, &games);
    assert_eq!(entries.iter().filter(|e| e.game_id == 2).count(), 1);
    assert!(entries[0].detail.ends_with("+1 mod"), "{}", entries[0].detail);
    let _ = AppStatus::Installed;
}

#[test]
fn nothing_going_on_means_an_empty_sidebar_section() {
    let mut games = sample_games();
    for g in &mut games {
        if g.status == AppStatus::UpdateReady {
            g.status = AppStatus::Installed;
        }
    }
    assert!(sidebar_entries(&ActivityBoard::new(), &games).is_empty());
}
