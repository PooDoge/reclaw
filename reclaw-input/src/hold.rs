//! Press-and-hold on a button, as a state machine with the time passed in. The mapper uses it so a
//! button can mean one thing on a tap and another when held (a toast's "hold X for details" while X
//! still opens Options when tapped), and it is exact in tests because nothing here reads a clock.
use std::time::{Duration, Instant};

use crate::action::Button;

/// A button that can be held, and for how long.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HoldRule {
    pub button: Button,
    pub after: Duration,
}

/// Where a hold is, as the UI hears about it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum HoldPhase {
    /// The button went down: start drawing the ring.
    Started,
    /// Held long enough: do the held thing. The release that follows does nothing.
    Completed,
    /// Let go early, or something else took over: clear the ring. A tap is reported separately,
    /// as the button's ordinary action.
    Cancelled,
}

/// What releasing a button meant.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Released {
    /// Let go before the time: it was a tap, do the button's ordinary action.
    Tap,
    /// The hold had already completed.
    AfterHold,
    /// That button was not being held.
    Nothing,
}

#[derive(Clone, Debug)]
pub struct HoldTracker {
    rules: Vec<HoldRule>,
    held: Option<Held>,
}

#[derive(Clone, Copy, Debug)]
struct Held {
    button: Button,
    since: Instant,
    fired: bool,
}

impl HoldTracker {
    pub fn new(rules: Vec<HoldRule>) -> Self {
        Self { rules, held: None }
    }

    pub fn rule(&self, button: Button) -> Option<&HoldRule> {
        self.rules.iter().find(|r| r.button == button)
    }

    /// The button went down. Returns whether a hold began: not for a button with no rule, and not
    /// while another is already held.
    pub fn press(&mut self, button: Button, now: Instant) -> bool {
        if self.held.is_some() || self.rule(button).is_none() {
            return false;
        }
        self.held = Some(Held { button, since: now, fired: false });
        true
    }

    /// Advance to `now`. Returns the button the moment its hold completes, once.
    pub fn tick(&mut self, now: Instant) -> Option<Button> {
        let held = self.held.as_mut()?;
        let after = self.rules.iter().find(|r| r.button == held.button)?.after;
        if !held.fired && now.saturating_duration_since(held.since) >= after {
            held.fired = true;
            return Some(held.button);
        }
        None
    }

    pub fn release(&mut self, button: Button) -> Released {
        match self.held {
            Some(held) if held.button == button => {
                self.held = None;
                if held.fired { Released::AfterHold } else { Released::Tap }
            }
            _ => Released::Nothing,
        }
    }

    /// Abandon a hold that has not completed. Returns its button, if there was one, so the caller can
    /// tell the UI to clear the ring.
    pub fn cancel(&mut self) -> Option<Button> {
        self.held.take().filter(|h| !h.fired).map(|h| h.button)
    }

    /// The button being held and how far along it is (0 to 1).
    pub fn progress(&self, now: Instant) -> Option<(Button, f32)> {
        let held = self.held?;
        let after = self.rule(held.button)?.after.as_secs_f32().max(f32::EPSILON);
        Some((held.button, (now.saturating_duration_since(held.since).as_secs_f32() / after).clamp(0., 1.)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    fn tracker() -> (HoldTracker, Instant) {
        let rules = vec![HoldRule { button: Button::West, after: ms(900) }, HoldRule { button: Button::North, after: ms(1200) }];
        (HoldTracker::new(rules), Instant::now())
    }

    #[test]
    fn a_hold_completes_once_after_its_time_and_the_release_does_nothing() {
        let (mut h, t) = tracker();
        assert!(h.press(Button::West, t));
        assert_eq!(h.tick(t + ms(899)), None);
        assert_eq!(h.tick(t + ms(900)), Some(Button::West));
        assert_eq!(h.tick(t + ms(2000)), None, "exactly once");
        assert_eq!(h.release(Button::West), Released::AfterHold);
    }

    #[test]
    fn releasing_early_is_a_tap() {
        let (mut h, t) = tracker();
        h.press(Button::North, t);
        assert_eq!(h.tick(t + ms(1000)), None);
        assert_eq!(h.release(Button::North), Released::Tap);
        assert_eq!(h.release(Button::North), Released::Nothing);
    }

    #[test]
    fn only_buttons_with_a_rule_hold_and_only_one_at_a_time() {
        let (mut h, t) = tracker();
        assert!(!h.press(Button::South, t), "no rule for South");
        assert!(h.press(Button::West, t));
        assert!(!h.press(Button::North, t + ms(100)), "West is already held");
        assert_eq!(h.tick(t + ms(950)), Some(Button::West));
    }

    #[test]
    fn progress_is_the_share_of_the_time_needed() {
        let (mut h, t) = tracker();
        assert_eq!(h.progress(t), None);
        h.press(Button::North, t);
        let (button, p) = h.progress(t + ms(600)).expect("held");
        assert_eq!(button, Button::North);
        assert!((p - 0.5).abs() < 1e-5);
        assert_eq!(h.progress(t + ms(9000)).map(|(_, p)| p), Some(1.));
    }

    #[test]
    fn cancel_reports_an_unfinished_hold_and_forgets_it() {
        let (mut h, t) = tracker();
        h.press(Button::West, t);
        assert_eq!(h.cancel(), Some(Button::West));
        assert_eq!(h.cancel(), None);
        assert_eq!(h.release(Button::West), Released::Nothing);
        h.press(Button::West, t);
        h.tick(t + ms(900));
        assert_eq!(h.cancel(), None, "a completed hold has nothing to clear");
    }
}
