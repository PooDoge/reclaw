//! Notifications in Deck mode: the toast, the hold gestures that act on it, and the details view.
use reclaw_input::{Button, HoldPhase};

use super::support::*;
use crate::notices::{Notice, NoticeKind};

fn hold(button: Button, phase: HoldPhase) -> Action {
    Action::Hold(button, phase)
}

fn with_notice(f: &mut Fixture) -> u64 {
    f.notices.push(Notice::update_finished(2, "Skyward Quest", None).details(vec!["Fixes saves".into(), "Faster loads".into()]))
}

#[test]
fn the_pad_reader_is_asked_for_holds_exactly_while_a_toast_is_up() {
    let mut f = Fixture::new();
    let mut s = f.state();
    assert!(s.sync(&f.view()).is_empty(), "no notice, nothing to ask");
    let id = with_notice(&mut f);
    assert!(s.sync(&f.view()).contains(&Effect::NoticeHolds(true)));
    assert!(s.sync(&f.view()).is_empty(), "asked once");
    f.notices.dismiss(id);
    assert!(s.sync(&f.view()).contains(&Effect::NoticeHolds(false)));
}

#[test]
fn holding_x_draws_the_ring_and_completing_it_opens_the_details() {
    let mut f = Fixture::new();
    let id = with_notice(&mut f);
    let mut s = f.state();
    s.sync(&f.view());
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    assert_eq!(s.holding(), Some(Button::West));
    s.apply(hold(Button::West, HoldPhase::Completed), &f.view());
    assert_eq!((s.holding(), s.overlay()), (None, Overlay::Notice(id)));
    assert_eq!(s.focus(), ids::NOTICE_CLOSE, "starts on the harmless choice");
    assert!(!s.toast_visible(&f.view()), "the toast gives way to the details");
}

#[test]
fn letting_go_early_clears_the_ring_and_opens_nothing() {
    let mut f = Fixture::new();
    with_notice(&mut f);
    let mut s = f.state();
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    s.apply(hold(Button::West, HoldPhase::Cancelled), &f.view());
    assert_eq!((s.holding(), s.overlay()), (None, Overlay::None));
}

#[test]
fn holding_y_dismisses_every_notification() {
    let mut f = Fixture::new();
    with_notice(&mut f);
    let mut s = f.state();
    s.apply(hold(Button::North, HoldPhase::Started), &f.view());
    let fx = s.apply(hold(Button::North, HoldPhase::Completed), &f.view());
    assert!(fx.contains(&Effect::DismissAllNotices), "{fx:?}");
}

#[test]
fn a_hold_with_no_toast_or_a_late_completion_does_nothing() {
    let mut f = Fixture::new();
    let mut s = f.state();
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    assert_eq!(s.holding(), None, "nothing is on screen to hold for");
    with_notice(&mut f);
    let mut s = f.state();
    let fx = s.apply(hold(Button::North, HoldPhase::Completed), &f.view());
    assert!(!fx.contains(&Effect::DismissAllNotices) && s.overlay() == Overlay::None, "a completion without a start is ignored: {fx:?}");
    s.apply(hold(Button::South, HoldPhase::Started), &f.view());
    assert_eq!(s.holding(), None, "only X and Y are holds");
}

#[test]
fn the_details_can_dismiss_the_notice_or_just_close() {
    let mut f = Fixture::new();
    let id = with_notice(&mut f);
    let mut s = f.state();
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    s.apply(hold(Button::West, HoldPhase::Completed), &f.view());
    // Closing the details brings the toast back, so the reader is asked for holds again.
    assert_eq!(press(&mut s, &f, &[Back]), vec![Effect::NoticeHolds(true)]);
    assert_eq!(s.overlay(), Overlay::None, "B closes the details and leaves the notice");

    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    s.apply(hold(Button::West, HoldPhase::Completed), &f.view());
    let fx = press(&mut s, &f, &[go(Right), Confirm]);
    assert!(fx.contains(&Effect::DismissNotice(id)), "{fx:?}");
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn details_open_when_the_notice_is_dismissed_elsewhere_close() {
    let mut f = Fixture::new();
    let id = with_notice(&mut f);
    let mut s = f.state();
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    s.apply(hold(Button::West, HoldPhase::Completed), &f.view());
    f.notices.dismiss(id);
    s.sync(&f.view());
    assert_eq!(s.overlay(), Overlay::None);
}

#[test]
fn the_side_panels_do_not_open_over_the_details() {
    let mut f = Fixture::new();
    with_notice(&mut f);
    let mut s = f.state();
    s.apply(hold(Button::West, HoldPhase::Started), &f.view());
    s.apply(hold(Button::West, HoldPhase::Completed), &f.view());
    press(&mut s, &f, &[MainMenu, QuickAccess]);
    assert!(matches!(s.overlay(), Overlay::Notice(_)));
    let _ = NoticeKind::UpdateFinished;
}
