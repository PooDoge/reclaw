use super::{a_mod, progress, started};
use crate::activity::*;

fn board_with(events: Vec<ActivityEvent>) -> ActivityBoard {
    let mut board = ActivityBoard::new();
    for e in events {
        board.apply(e);
    }
    board
}

#[test]
fn a_job_runs_from_queued_to_finished() {
    let mut board = ActivityBoard::new();
    assert_eq!(board.apply(started(1, 2, Kind::Update, Some(1000))), Some((2, Change::Started)));
    assert_eq!(board.get(1).map(|a| a.stage), Some(Stage::Queued));
    assert_eq!(board.apply(progress(1, Stage::Downloading, 250, Some(1000), Some(100))), Some((2, Change::Progressed)));
    let a = board.get(1).expect("job");
    assert_eq!((a.percent(), a.eta().map(|d| d.as_secs())), (Some(25), Some(8)));
    let changelog = Changelog { from: "v1".into(), to: "v2".into(), notes: "Fixes saves".into(), url: None };
    assert_eq!(board.apply(ActivityEvent::Finished { id: 1, changelog: Some(changelog.clone()) }), Some((2, Change::Finished)));
    let a = board.get(1).expect("still there until the app restarts");
    assert_eq!((a.outcome.clone(), a.progress(), a.changelog.clone()), (Outcome::Finished, Some(1.), Some(changelog)));
}

#[test]
fn late_duplicate_and_unknown_events_are_ignored() {
    let mut board = board_with(vec![started(1, 2, Kind::Install, None)]);
    assert_eq!(board.apply(started(1, 9, Kind::Install, None)), None, "a repeated start does not add a second row");
    assert_eq!(board.apply(progress(77, Stage::Downloading, 1, None, None)), None);
    board.apply(ActivityEvent::Failed { id: 1, reason: "no space".into(), details: Vec::new() });
    assert_eq!(board.apply(progress(1, Stage::Downloading, 5, None, None)), None, "progress after the end is ignored");
    assert_eq!(board.apply(ActivityEvent::Finished { id: 1, changelog: None }), None, "an ended job cannot end again");
}

#[test]
fn bytes_never_pass_the_total_and_percent_never_says_100_while_running() {
    let mut board = board_with(vec![started(1, 1, Kind::Install, Some(100))]);
    board.apply(progress(1, Stage::Downloading, 5_000, Some(100), None));
    let a = board.get(1).expect("job");
    assert_eq!((a.bytes_done, a.percent()), (100, Some(99)));
    board.apply(ActivityEvent::Finished { id: 1, changelog: None });
    assert_eq!(board.get(1).and_then(|a| a.percent()), Some(100));
}

#[test]
fn time_left_is_only_offered_while_bytes_are_coming_in() {
    let mut board = board_with(vec![started(1, 1, Kind::Install, Some(1000))]);
    board.apply(progress(1, Stage::Downloading, 500, Some(1000), Some(50)));
    assert_eq!(board.get(1).and_then(|a| a.eta()).map(|d| d.as_secs()), Some(10));
    board.apply(progress(1, Stage::Finishing, 1000, Some(1000), Some(50)));
    assert_eq!(board.get(1).and_then(|a| a.eta()), None, "building has no honest estimate");
    board.apply(progress(1, Stage::Downloading, 10, Some(1000), Some(0)));
    assert_eq!(board.get(1).and_then(|a| a.eta()), None, "a stalled transfer has none either");
}

#[test]
fn cancelling_removes_a_running_job_and_dismiss_removes_an_ended_one() {
    let mut board = board_with(vec![started(1, 1, Kind::Install, None), started(2, 1, Kind::Update, None)]);
    assert_eq!(board.apply(ActivityEvent::Dismiss { id: 1 }), None, "a running job cannot be dismissed");
    assert_eq!(board.apply(ActivityEvent::Cancelled { id: 1 }), Some((1, Change::Removed)));
    board.apply(ActivityEvent::Finished { id: 2, changelog: None });
    assert_eq!(board.apply(ActivityEvent::Cancelled { id: 2 }), None, "an ended job is not cancelled");
    assert_eq!(board.apply(ActivityEvent::Dismiss { id: 2 }), Some((1, Change::Removed)));
    assert!(board.all().is_empty());
}

#[test]
fn the_queue_lists_running_then_failed_then_finished_newest_first() {
    let mut board = board_with(vec![
        started(1, 1, Kind::Update, None),
        started(2, 2, Kind::Update, None),
        started(3, 3, Kind::Update, None),
        started(4, 4, Kind::Update, None),
    ]);
    board.apply(ActivityEvent::Finished { id: 1, changelog: None });
    board.apply(ActivityEvent::Failed { id: 2, reason: "x".into(), details: Vec::new() });
    board.apply(ActivityEvent::Finished { id: 3, changelog: None });
    let ids: Vec<u64> = board.queue().iter().map(|a| a.id).collect();
    assert_eq!(ids, vec![4, 2, 3, 1]);
    assert_eq!(board.counts(), Counts { running: 1, failed: 1, finished: 2 });
}

#[test]
fn only_the_newest_ended_rows_are_kept_and_running_ones_never_dropped() {
    let mut board = ActivityBoard::new();
    board.apply(started(1_000, 1, Kind::Install, None));
    for id in 0..60 {
        board.apply(started(id, 1, a_mod(), None));
        board.apply(ActivityEvent::Finished { id, changelog: None });
    }
    assert_eq!(board.counts().finished, 50);
    assert!(board.get(0).is_none() && board.get(59).is_some());
    assert!(board.get(1_000).is_some());
}
