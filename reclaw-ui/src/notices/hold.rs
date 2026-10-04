use std::time::Duration;

/// How long each held button must be held. Long enough that nobody triggers it by accident while
/// pressing the button for its normal job, short enough not to feel like a chore.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HoldTimes {
    /// Show more about the notification.
    pub details: Duration,
    /// Dismiss every notification.
    pub dismiss_all: Duration,
}

impl Default for HoldTimes {
    fn default() -> Self {
        Self { details: Duration::from_millis(900), dismiss_all: Duration::from_millis(1200) }
    }
}

/// What a completed hold asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HoldAction {
    Details,
    DismissAll,
}

/// Where a hold is, for drawing the ring around the button glyph.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum HoldPhase {
    Idle,
    /// Held for part of the time; `progress` is 0 to 1.
    Holding {
        action: HoldAction,
        progress: f32,
    },
}

/// A press-and-hold on a button, as a state machine. The caller passes the time in (`elapsed` since
/// some fixed start) so the logic is exact in tests and the UI can drive it from a frame timer.
///
/// A short press is not a hold: [`release`](Self::release) tells the caller so, and the button's
/// ordinary action should then run. That is why the ordinary action must wait for the release while
/// a notification is on screen.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Hold {
    times: HoldTimes,
    held: Option<(HoldAction, Duration)>,
    fired: bool,
}

/// What releasing the button means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Released {
    /// Released early: it was a tap, run the button's normal action.
    Tap,
    /// The hold had already completed; nothing more to do.
    AfterHold,
    /// The button was not being held (a stray release).
    Nothing,
}

impl Hold {
    pub fn new(times: HoldTimes) -> Self {
        Self { times, held: None, fired: false }
    }

    fn needed(&self, action: HoldAction) -> Duration {
        match action {
            HoldAction::Details => self.times.details,
            HoldAction::DismissAll => self.times.dismiss_all,
        }
    }

    /// The button went down at `now`. A second press while one is held is ignored.
    pub fn press(&mut self, action: HoldAction, now: Duration) {
        if self.held.is_none() {
            self.held = Some((action, now));
            self.fired = false;
        }
    }

    /// Advance to `now`. Returns the action exactly once, the moment the hold completes.
    pub fn tick(&mut self, now: Duration) -> Option<HoldAction> {
        let (action, since) = self.held?;
        if !self.fired && now.saturating_sub(since) >= self.needed(action) {
            self.fired = true;
            return Some(action);
        }
        None
    }

    pub fn release(&mut self) -> Released {
        match self.held.take() {
            None => Released::Nothing,
            Some(_) if self.fired => Released::AfterHold,
            Some(_) => Released::Tap,
        }
    }

    /// Abandon the hold without a tap (the notification went away, the window lost focus).
    pub fn cancel(&mut self) {
        self.held = None;
        self.fired = false;
    }

    pub fn is_held(&self) -> bool {
        self.held.is_some()
    }

    pub fn phase(&self, now: Duration) -> HoldPhase {
        match self.held {
            None => HoldPhase::Idle,
            Some((action, since)) => {
                let needed = self.needed(action).as_secs_f32().max(f32::EPSILON);
                HoldPhase::Holding { action, progress: (now.saturating_sub(since).as_secs_f32() / needed).clamp(0., 1.) }
            }
        }
    }
}
