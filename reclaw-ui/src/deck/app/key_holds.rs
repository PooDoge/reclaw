//! Holding X or Y on a keyboard, for Deck mode without a pad: the same gesture the pad reader runs
//! for a controller, built on the same [`HoldTracker`] so a tap and a hold mean the same on both.
//!
//! Pure: the clock is passed in, so the timing is exact in tests. `Dispatcher` owns the timer.
use std::time::Instant;

use reclaw_input::{Action, Button, HoldPhase, HoldTracker, Released};

use crate::notices::hold_rules;

/// The key that stands for a pad button that can be held.
pub(super) fn button_for_key(key: &str) -> Option<Button> {
    match key {
        "x" => Some(Button::West),
        "y" => Some(Button::North),
        _ => None,
    }
}

/// What a tap of the button does, as the pad's default mapping has it (X: Options, Y: Search).
fn tap_action(button: Button) -> Option<Action> {
    match button {
        Button::West => Some(Action::Secondary),
        Button::North => Some(Action::Tertiary),
        _ => None,
    }
}

pub(super) struct KeyHolds {
    tracker: HoldTracker,
    /// A notification is up, so holding means something. Otherwise the keys are plain taps.
    enabled: bool,
    /// The held key has not completed yet, so a timer should still be ticking.
    waiting: bool,
    /// Keys that went down and have not come up. The system repeats a held key as more "key down" events
    /// (Freya does not say which are repeats), and a pad never does, so a second down with no up between
    /// is not a press. Without this, holding Y past its time dismisses the toast and the repeats that
    /// follow, now that holding is off, would each be a Search.
    down: Vec<Button>,
}

impl KeyHolds {
    pub fn new() -> Self {
        Self { tracker: HoldTracker::new(hold_rules()), enabled: false, waiting: false, down: Vec::new() }
    }

    /// Turn holding on or off as notifications come and go. Turning it off mid-hold abandons the
    /// hold and returns the action that clears the ring.
    pub fn set_enabled(&mut self, on: bool) -> Option<Action> {
        self.enabled = on;
        if on {
            return None;
        }
        self.waiting = false;
        self.tracker.cancel().map(|button| Action::Hold(button, HoldPhase::Cancelled))
    }

    /// Whether a timer should be ticking: a hold has begun and not yet completed.
    pub fn waiting(&self) -> bool {
        self.waiting
    }

    /// The key went down. Actions to apply, in order.
    pub fn down(&mut self, button: Button, now: Instant) -> Vec<Action> {
        if self.down.contains(&button) {
            return Vec::new();
        }
        self.down.push(button);
        if !self.enabled {
            return tap_action(button).into_iter().collect();
        }
        // A second key while one is held starts nothing. (A repeat of the same key never gets here.)
        if self.tracker.press(button, now) {
            self.waiting = true;
            vec![Action::Hold(button, HoldPhase::Started)]
        } else {
            Vec::new()
        }
    }

    /// Advance the clock. The completed hold, once.
    pub fn tick(&mut self, now: Instant) -> Option<Action> {
        let button = self.tracker.tick(now)?;
        self.waiting = false;
        Some(Action::Hold(button, HoldPhase::Completed))
    }

    /// The key came up: a tap clears the ring and then does the button's ordinary job; a release
    /// after a completed hold does nothing.
    pub fn release(&mut self, button: Button) -> Vec<Action> {
        self.down.retain(|b| *b != button);
        match self.tracker.release(button) {
            Released::Tap => {
                self.waiting = false;
                let mut actions = vec![Action::Hold(button, HoldPhase::Cancelled)];
                actions.extend(tap_action(button));
                actions
            }
            Released::AfterHold | Released::Nothing => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::notices::{DETAILS_AFTER, DISMISS_ALL_AFTER};

    fn on() -> (KeyHolds, Instant) {
        let mut holds = KeyHolds::new();
        assert_eq!(holds.set_enabled(true), None);
        (holds, Instant::now())
    }

    #[test]
    fn only_x_and_y_are_hold_keys() {
        assert_eq!(button_for_key("x"), Some(Button::West));
        assert_eq!(button_for_key("y"), Some(Button::North));
        assert_eq!(button_for_key("z"), None);
    }

    #[test]
    fn without_a_notification_the_keys_are_plain_taps() {
        let mut holds = KeyHolds::new();
        let t = Instant::now();
        assert_eq!(holds.down(Button::West, t), vec![Action::Secondary]);
        assert_eq!(holds.down(Button::North, t), vec![Action::Tertiary]);
        assert!(holds.release(Button::West).is_empty(), "nothing was held");
        assert!(!holds.waiting());
    }

    #[test]
    fn holding_x_starts_then_completes_then_the_release_is_silent() {
        let (mut holds, t) = on();
        assert_eq!(holds.down(Button::West, t), vec![Action::Hold(Button::West, HoldPhase::Started)]);
        assert!(holds.waiting());
        assert_eq!(holds.tick(t + DETAILS_AFTER - Duration::from_millis(1)), None);
        assert_eq!(holds.tick(t + DETAILS_AFTER), Some(Action::Hold(Button::West, HoldPhase::Completed)));
        assert!(!holds.waiting(), "the timer can stop once it has fired");
        assert!(holds.release(Button::West).is_empty());
    }

    #[test]
    fn a_tap_while_a_notification_is_up_clears_the_ring_and_still_acts() {
        let (mut holds, t) = on();
        holds.down(Button::North, t);
        assert_eq!(holds.tick(t + Duration::from_millis(300)), None);
        assert_eq!(holds.release(Button::North), vec![Action::Hold(Button::North, HoldPhase::Cancelled), Action::Tertiary]);
        assert!(!holds.waiting());
    }

    #[test]
    fn key_repeat_and_a_second_key_do_not_restart_the_hold() {
        let (mut holds, t) = on();
        assert_eq!(holds.down(Button::West, t).len(), 1);
        assert!(holds.down(Button::West, t + Duration::from_millis(400)).is_empty(), "OS key repeat");
        assert!(holds.down(Button::North, t + Duration::from_millis(500)).is_empty(), "one hold at a time");
        // The clock still runs from the first press.
        assert_eq!(holds.tick(t + DETAILS_AFTER), Some(Action::Hold(Button::West, HoldPhase::Completed)));
        assert!(DISMISS_ALL_AFTER > DETAILS_AFTER);
    }

    #[test]
    fn the_notification_going_away_mid_hold_clears_the_ring() {
        let (mut holds, t) = on();
        holds.down(Button::West, t);
        assert_eq!(holds.set_enabled(false), Some(Action::Hold(Button::West, HoldPhase::Cancelled)));
        assert!(!holds.waiting());
        assert!(holds.release(Button::West).is_empty(), "the key coming up afterwards is not a tap");
        assert_eq!(holds.set_enabled(false), None);
    }

    #[test]
    fn key_repeat_after_the_hold_fired_and_the_notification_went_is_not_a_tap() {
        let (mut holds, t) = on();
        holds.down(Button::North, t);
        assert_eq!(holds.tick(t + DISMISS_ALL_AFTER), Some(Action::Hold(Button::North, HoldPhase::Completed)));
        // Dismissing the last notification turns holding off while the key is still down.
        holds.set_enabled(false);
        let repeat = t + DISMISS_ALL_AFTER + Duration::from_millis(40);
        assert!(holds.down(Button::North, repeat).is_empty(), "the system repeating a held key is not a new press");
        assert!(holds.release(Button::North).is_empty());
        assert_eq!(holds.down(Button::North, repeat + Duration::from_secs(1)), vec![Action::Tertiary], "a fresh press is a tap again");
    }

    #[test]
    fn a_plain_tap_key_does_not_repeat_while_it_is_held() {
        let mut holds = KeyHolds::new();
        let t = Instant::now();
        assert_eq!(holds.down(Button::West, t), vec![Action::Secondary]);
        assert!(holds.down(Button::West, t + Duration::from_millis(600)).is_empty(), "held, not pressed again");
        assert!(holds.release(Button::West).is_empty());
        assert_eq!(holds.down(Button::West, t + Duration::from_secs(2)), vec![Action::Secondary]);
    }
}
