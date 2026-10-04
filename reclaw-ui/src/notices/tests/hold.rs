use std::time::Duration;

use crate::notices::{Hold, HoldAction, HoldPhase, HoldTimes, Released};

fn ms(n: u64) -> Duration {
    Duration::from_millis(n)
}

fn hold() -> Hold {
    Hold::new(HoldTimes { details: ms(900), dismiss_all: ms(1200) })
}

#[test]
fn a_hold_completes_once_after_its_time() {
    let mut h = hold();
    h.press(HoldAction::Details, ms(1000));
    assert_eq!(h.tick(ms(1500)), None);
    assert_eq!(h.tick(ms(1900)), Some(HoldAction::Details));
    assert_eq!(h.tick(ms(2500)), None, "it fires exactly once");
    assert_eq!(h.release(), Released::AfterHold);
}

#[test]
fn releasing_early_is_a_tap() {
    let mut h = hold();
    h.press(HoldAction::DismissAll, ms(0));
    assert_eq!(h.tick(ms(1000)), None);
    assert_eq!(h.release(), Released::Tap);
    assert_eq!(h.release(), Released::Nothing);
    assert!(!h.is_held());
}

#[test]
fn the_ring_fills_in_proportion_to_the_time_needed() {
    let mut h = hold();
    assert_eq!(h.phase(ms(0)), HoldPhase::Idle);
    h.press(HoldAction::Details, ms(100));
    assert_eq!(h.phase(ms(100)), HoldPhase::Holding { action: HoldAction::Details, progress: 0. });
    let HoldPhase::Holding { progress, .. } = h.phase(ms(550)) else { panic!("holding") };
    assert!((progress - 0.5).abs() < 1e-5);
    let HoldPhase::Holding { progress, .. } = h.phase(ms(5000)) else { panic!("holding") };
    assert_eq!(progress, 1.);
}

#[test]
fn a_second_button_pressed_while_one_is_held_is_ignored() {
    let mut h = hold();
    h.press(HoldAction::Details, ms(0));
    h.press(HoldAction::DismissAll, ms(100));
    assert_eq!(h.tick(ms(950)), Some(HoldAction::Details));
}

#[test]
fn cancel_forgets_the_hold_without_a_tap() {
    let mut h = hold();
    h.press(HoldAction::Details, ms(0));
    h.cancel();
    assert_eq!(h.release(), Released::Nothing);
    assert_eq!(h.tick(ms(5000)), None);
}
