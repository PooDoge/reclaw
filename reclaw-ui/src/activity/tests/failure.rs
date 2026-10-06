use super::started;
use crate::activity::*;

fn failed(id: ActivityId, reason: &str) -> ActivityEvent {
    ActivityEvent::Failed { id, reason: reason.into(), details: vec!["the cause".into()] }
}

#[test]
fn a_short_reason_is_kept_whole_and_a_long_one_is_cut_at_a_word() {
    assert_eq!(short_reason("Disk full"), "Disk full");
    assert_eq!(short_reason("\n  First line\nsecond line"), "First line");
    let long = "This release's download for Linux-X64 is a Windows installer, which Reclaw does not install, so pick another release.";
    let short = short_reason(long);
    assert!(short.chars().count() <= HINT_CHARS && short.ends_with("...") && !short.contains("  "), "{short}");
    assert!(long.starts_with(short.trim_end_matches("...")), "cut at a word, nothing rewritten: {short}");
    assert!(!short.trim_end_matches("...").ends_with(','), "{short}");
}

#[test]
fn one_long_word_is_cut_where_it_must_be() {
    let path = format!("/home/you/{}", "x".repeat(200));
    let short = short_reason(&path);
    assert_eq!(short.chars().count(), HINT_CHARS);
    assert!(short.starts_with("/home/you/xxx") && short.ends_with("..."));
}

#[test]
fn text_beyond_ascii_is_cut_on_a_character() {
    let short = short_reason(&"é".repeat(200));
    assert_eq!(short.chars().count(), HINT_CHARS);
}

#[test]
fn a_game_hint_names_its_newest_failure_and_the_report_carries_the_log() {
    let mut board = ActivityBoard::new();
    board.apply(started(1, 7, Kind::Install, None));
    board.apply(failed(1, "old reason"));
    board.apply(started(2, 7, Kind::Install, None));
    assert_eq!(board.apply(ActivityEvent::Log { id: 2, lines: vec!["a line".into()] }), Some((7, Change::Progressed)));
    board.apply(failed(2, "new reason"));

    assert_eq!(hint_for_game(&board, 7), FailureHint { activity: Some(2), text: "new reason".into() });
    let report = report(&board, 2).expect("a report");
    assert_eq!(
        (report.reason.as_str(), report.details.clone(), report.log.clone()),
        ("new reason", vec!["the cause".to_string()], vec!["a line".to_string()])
    );
    assert_eq!(report.title, "job 2");
}

#[test]
fn a_failure_from_an_earlier_run_has_no_job_to_open() {
    let board = ActivityBoard::new();
    assert_eq!(hint_for_game(&board, 3), FailureHint { activity: None, text: EARLIER_FAILURE.into() });
    assert_eq!(report(&board, 3), None);
}

#[test]
fn a_running_or_finished_job_has_no_report_and_a_log_for_an_unknown_job_is_ignored() {
    let mut board = ActivityBoard::new();
    board.apply(started(1, 1, Kind::Install, None));
    assert_eq!(report(&board, 1), None);
    assert_eq!(board.apply(ActivityEvent::Log { id: 9, lines: vec![] }), None);
    board.apply(ActivityEvent::Finished { id: 1, changelog: None });
    assert_eq!(report(&board, 1), None);
}

#[test]
fn a_line_is_toned_by_its_level_not_by_its_words() {
    assert_eq!(line_tone("05:42:00.123  WARN reclaw_net::net: request failed"), LineTone::Warning);
    assert_eq!(line_tone("05:42:00.123 ERROR reclaw_app: it broke"), LineTone::Error);
    assert_eq!(line_tone("05:42:00.123  INFO reclaw_app: the word ERROR in a message"), LineTone::Plain);
    assert_eq!(line_tone("(5 earlier lines not kept)"), LineTone::Plain);
}
