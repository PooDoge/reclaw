//! Deck mode's notifications: the toast, the holds that act on it (from the pad reader and from a
//! keyboard), the details view, and the progress on game cards.
//!
//! The keyboard holds run on a real clock (the headless runner polls in real time), so the tests
//! that wait for one take about a second each.
use reclaw_input::{Action, Button, HoldPhase};
use reclaw_ui::{
    activity::{ActivityEvent, Changelog},
    effect::Effect,
    store::AppAction,
};

use crate::common::*;

/// The Skyward Quest update in the sample finishes, with release notes.
fn finish_update(s: &mut Session) {
    let changelog =
        Changelog { from: "v0.9.1".into(), to: "v0.9.2".into(), notes: "Fixed the water level\nNew photo mode".into(), url: None };
    s.dispatch(AppAction::Activity(ActivityEvent::Finished { id: 1, changelog: Some(changelog) }));
}

fn hold(s: &mut Session, button: Button, phase: HoldPhase) {
    s.pad(Action::Hold(button, phase));
}

#[test]
fn nothing_is_shown_until_something_happens() {
    let s = Mount::deck().start();
    assert!(!s.has_label("Hold for details"), "{:?}", s.labels());
    assert!(!s.has_label("Skyward Quest updated"));
}

#[test]
fn a_finished_update_raises_a_toast_and_turns_the_holds_on() {
    let mut s = Mount::deck().start();
    s.take_effects();
    finish_update(&mut s);
    assert!(s.has_label("Skyward Quest updated"), "{:?}", s.labels());
    assert!(s.has_label("v0.9.1 to v0.9.2"));
    assert!(s.has_label("Hold for details") && s.has_label("Hold to dismiss all"));
    assert!(s.effects().contains(&Effect::NoticeHolds(true)), "the pad reader is told to watch for holds: {:?}", s.effects());
}

#[test]
fn holding_x_on_the_pad_opens_the_details_and_close_returns_to_the_toast() {
    let mut s = Mount::deck().start();
    finish_update(&mut s);

    hold(&mut s, Button::West, HoldPhase::Started);
    assert!(s.has_label("Hold for details"), "the toast stays while the ring fills");
    hold(&mut s, Button::West, HoldPhase::Completed);
    assert!(s.has_label("Fixed the water level"), "the release notes are listed: {:?}", s.labels());
    assert!(s.has_label("New photo mode"));
    assert!(s.has_label("Close") && s.has_label("Dismiss"));
    assert!(!s.has_label("Hold for details"), "the toast gives way to the details");

    s.press(freya::prelude::NamedKey::Escape);
    assert!(!s.has_label("Fixed the water level"));
    assert!(s.has_label("Hold for details"), "the notice is still there: dismissing is a choice");
}

#[test]
fn dismissing_from_the_details_removes_the_notice_and_the_toast() {
    let mut s = Mount::deck().start();
    finish_update(&mut s);
    hold(&mut s, Button::West, HoldPhase::Completed);
    // Completed without Started is ignored: a hold must have begun.
    assert!(!s.has_label("Fixed the water level"));

    hold(&mut s, Button::West, HoldPhase::Started);
    hold(&mut s, Button::West, HoldPhase::Completed);
    s.take_effects();
    s.press(freya::prelude::NamedKey::ArrowRight); // Close -> Dismiss
    s.press(freya::prelude::NamedKey::Enter);
    assert!(s.effects().iter().any(|e| matches!(e, Effect::DismissNotice(_))), "{:?}", s.effects());
    assert!(!s.has_label("Skyward Quest updated"));
    assert!(s.effects().contains(&Effect::NoticeHolds(false)), "nothing left to hold: {:?}", s.effects());
}

#[test]
fn holding_y_dismisses_every_notification() {
    let mut s = Mount::deck().start();
    finish_update(&mut s);
    s.dispatch(AppAction::Activity(ActivityEvent::Failed { id: 3, reason: "Disk full".into() }));
    assert!(s.has_label("Ghost Data Pack failed"), "the newest notice is the one shown: {:?}", s.labels());
    s.take_effects();

    hold(&mut s, Button::North, HoldPhase::Started);
    hold(&mut s, Button::North, HoldPhase::Completed);
    assert!(s.effects().contains(&Effect::DismissAllNotices), "{:?}", s.effects());
    assert!(!s.has_label("Hold to dismiss all"), "no toast is left");
    assert!(s.effects().contains(&Effect::NoticeHolds(false)), "and X and Y go back to being plain buttons: {:?}", s.effects());
    assert!(!s.has_label("Skyward Quest updated"), "the older notice went too");
}

#[test]
fn letting_go_early_cancels_the_hold() {
    let mut s = Mount::deck().start();
    finish_update(&mut s);
    hold(&mut s, Button::North, HoldPhase::Started);
    hold(&mut s, Button::North, HoldPhase::Cancelled);
    hold(&mut s, Button::North, HoldPhase::Completed); // a stray completion after a cancel does nothing
    assert!(s.has_label("Skyward Quest updated"), "still there: {:?}", s.labels());
    assert!(!s.effects().contains(&Effect::DismissAllNotices));
}

#[test]
fn holding_x_on_a_keyboard_takes_about_a_second_and_a_tap_still_opens_options() {
    let mut s = Mount::deck().start();
    finish_update(&mut s);

    // A tap: down and up at once. Not long enough, so it is X's ordinary job (Options).
    s.key_down_char("x");
    s.key_up_char("x");
    assert!(s.has_label("Manage"), "the Options menu opened: {:?}", s.labels());
    s.press(freya::prelude::NamedKey::Escape);
    assert!(!s.has_label("Manage"));

    // A hold: still down after the time has passed.
    s.key_down_char("x");
    s.pump(1100);
    s.runner.sync_and_update();
    assert!(s.has_label("Fixed the water level"), "the details opened: {:?}", s.labels());
    s.key_up_char("x");
    assert!(s.has_label("Fixed the water level"), "letting go after a completed hold does nothing more");
    assert!(!s.has_label("Manage"));
}

#[test]
fn without_a_notification_x_and_y_are_just_taps() {
    let mut s = Mount::deck().start();
    s.take_effects();
    s.key_down_char("y");
    s.key_up_char("y");
    assert!(s.effects().contains(&Effect::Search), "Y is Search: {:?}", s.effects());
    s.key_down_char("x");
    s.key_up_char("x");
    assert!(s.has_label("Manage"), "X is Options");
}

#[test]
fn cards_show_what_their_games_are_doing() {
    let s = Mount::deck().start();
    // The sample has Skyward Quest downloading its update at 34%, and a mod downloading for Tide Racer.
    assert!(s.has_label("34%"), "{:?}", s.labels());
    assert!(s.has_label("Failed") || s.has_label("1 mod"), "{:?}", s.labels());
}

#[test]
fn snapshots() {
    let mut s = Mount::deck().start();
    s.snapshot("deck-cards-activity");

    finish_update(&mut s);
    s.snapshot("deck-toast");

    hold(&mut s, Button::West, HoldPhase::Started);
    s.pump(450);
    s.snapshot("deck-toast-holding");

    hold(&mut s, Button::West, HoldPhase::Completed);
    s.snapshot("deck-notice-details");

    let mut tv = Mount::deck().size(1920., 1080.).start();
    finish_update(&mut tv);
    tv.snapshot("deck-toast-tv");
}
