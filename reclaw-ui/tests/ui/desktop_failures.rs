//! A failed install explains itself: hovering "Failed" shows why in one line, pressing it shows the whole reason and the job's log.
//! The sample board has one failed job: Moon Garden's update (job 2, game 5).
use reclaw_ui::{
    activity::{ActivityEvent, EARLIER_FAILURE},
    effect::Effect,
    nav::Route,
    shell::{DevOverrides, MotionOverride},
    store::AppAction,
};

use freya::prelude::NamedKey;

use crate::common::*;

const REASON: &str = "Release asset not found. Check the repository or choose another version.";
const TITLE: &str = "Moon Garden v2.0.1 failed";

fn at(route: Route) -> Session {
    let dev = DevOverrides { motion: Some(MotionOverride::Reduced), ..DevOverrides::default() };
    let mut s = Mount::desktop().size(1100., 900.).dev(dev).start_at(route);
    s.dispatch(AppAction::Activity(ActivityEvent::Log {
        id: 2,
        lines: vec![
            "05:42:00.101  INFO reclaw_app::host::install::jobs: install started repo=\"o/moon\"".into(),
            "05:42:01.734  WARN reclaw_app::host::install::jobs: the install failed error=asset not found".into(),
        ],
    }));
    s
}

/// Rest the pointer on the middle of the label, past the tooltip's delay.
fn hover_label(s: &mut Session, text: &str) {
    let (l, t, r, b) = s.top_label_box(text).unwrap_or_else(|| panic!("no label {text:?} in {:?}", s.labels()));
    s.runner.move_cursor((f64::from((l + r) / 2.), f64::from((t + b) / 2.)));
    s.pump(700);
    s.runner.sync_and_update();
}

#[test]
fn hovering_failed_in_downloads_shows_the_reason_and_pressing_it_shows_the_log() {
    let mut s = at(Route::Downloads {});
    hover_label(&mut s, "Failed");
    // The reason is on the row already; the tooltip is a second label with the same (short enough) text.
    assert_eq!(s.labels().iter().filter(|l| *l == REASON).count(), 2, "{:?}", s.labels());
    s.snapshot("desktop-failed-tooltip");

    s.click_label("Failed");
    assert!(s.has_label(TITLE), "{:?}", s.labels());
    assert!(s.labels().iter().any(|l| l.contains("WARN") && l.contains("the install failed")), "{:?}", s.labels());
    assert!(s.has_label("Open log folder"));
    s.snapshot("desktop-failure-log");

    s.click_label("Open log folder");
    assert!(s.take_effects().contains(&Effect::OpenLogFolder));
    s.click_label("Close");
    assert!(!s.has_label(TITLE), "{:?}", s.labels());
}

#[test]
fn the_failed_badge_on_the_game_page_opens_the_same_report() {
    let mut s = at(Route::Game { id: 5 });
    hover_label(&mut s, "FAILED");
    assert!(s.has_label(REASON), "{:?}", s.labels());
    s.click_label("FAILED");
    assert!(s.has_label(TITLE), "{:?}", s.labels());
    s.press(NamedKey::Escape);
    s.settle();
    assert!(!s.has_label(TITLE), "Escape closes it before the page: {:?}", s.labels());
    assert!(s.has_label("Moon Garden"), "still on the game page");
}

#[test]
fn a_failure_from_an_earlier_run_says_so_and_opens_the_log_folder() {
    let mut s = at(Route::Game { id: 5 });
    s.dispatch(AppAction::Activity(ActivityEvent::Dismiss { id: 2 }));
    hover_label(&mut s, "FAILED");
    assert!(s.has_label(EARLIER_FAILURE), "{:?}", s.labels());
    s.take_effects();
    s.click_label("FAILED");
    assert!(s.take_effects().contains(&Effect::OpenLogFolder));
}
