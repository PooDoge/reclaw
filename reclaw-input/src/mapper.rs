use std::time::{Duration, Instant};

use crate::{
    action::{Action, ActionMap, Axis, Button, Direction, RawEvent},
    hold::{HoldPhase, HoldRule, HoldTracker, Released},
};

/// Who currently receives the gamepad. While an app runs, the launcher must not navigate its own
/// menus from the same button presses; only Guide is allowed through so the user can get back.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputOwner {
    #[default]
    Launcher,
    App,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct MapperConfig {
    /// A stick registers a direction when its dominant axis passes this...
    pub stick_enter: f32,
    /// ...and releases it only when it falls below this (hysteresis stops jitter near the edge).
    pub stick_exit: f32,
    pub repeat_delay: Duration,
    pub repeat_interval: Duration,
}

impl Default for MapperConfig {
    fn default() -> Self {
        Self { stick_enter: 0.6, stick_exit: 0.4, repeat_delay: Duration::from_millis(400), repeat_interval: Duration::from_millis(110) }
    }
}

/// Turns raw events into [`Action`]s. Call [`handle`](Self::handle) per event and
/// [`tick`](Self::tick) regularly (about every 10-16ms) so held directions repeat.
#[derive(Debug)]
pub struct InputMapper {
    config: MapperConfig,
    map: ActionMap,
    owner: InputOwner,
    dpad: [bool; 4],
    dpad_last: Option<Direction>,
    stick: (f32, f32),
    stick_dir: Option<Direction>,
    /// Direction currently repeating and when it fires next.
    repeating: Option<(Direction, Instant)>,
    holds: HoldTracker,
    /// Whether the hold rules apply. Off, every button acts on press as usual.
    holds_on: bool,
}

fn index(dir: Direction) -> usize {
    match dir {
        Direction::Up => 0,
        Direction::Down => 1,
        Direction::Left => 2,
        Direction::Right => 3,
    }
}

impl InputMapper {
    pub fn new(map: ActionMap, config: MapperConfig) -> Self {
        Self {
            config,
            map,
            owner: InputOwner::Launcher,
            dpad: [false; 4],
            dpad_last: None,
            stick: (0., 0.),
            stick_dir: None,
            repeating: None,
            holds: HoldTracker::new(Vec::new()),
            holds_on: false,
        }
    }

    pub fn owner(&self) -> InputOwner {
        self.owner
    }

    pub fn set_owner(&mut self, owner: InputOwner) {
        if owner != self.owner {
            self.owner = owner;
            self.reset();
        }
    }

    pub fn set_map(&mut self, map: ActionMap) {
        self.map = map;
    }

    /// Which buttons can be held, and for how long. They only act as holds while
    /// [`set_holds_on`](Self::set_holds_on) is true: a held button acts on release (a tap runs its
    /// ordinary action then) instead of on press, so it is only worth doing while something on screen
    /// offers the hold.
    pub fn set_hold_rules(&mut self, rules: Vec<HoldRule>) {
        self.holds = HoldTracker::new(rules);
    }

    /// Turn the hold rules on or off. Turning them off while a button is held clears its ring.
    pub fn set_holds_on(&mut self, on: bool) -> Vec<Action> {
        self.holds_on = on;
        if on {
            return Vec::new();
        }
        self.holds.cancel().map(|b| vec![Action::Hold(b, HoldPhase::Cancelled)]).unwrap_or_default()
    }

    fn reset(&mut self) {
        self.dpad = [false; 4];
        self.dpad_last = None;
        self.stick = (0., 0.);
        self.stick_dir = None;
        self.repeating = None;
        self.holds.cancel();
    }

    pub fn handle(&mut self, event: RawEvent, now: Instant) -> Vec<Action> {
        match event {
            RawEvent::Disconnected => {
                self.reset();
                Vec::new()
            }
            RawEvent::Button { button, pressed } => self.button(button, pressed, now),
            RawEvent::Axis { axis, value } => self.axis(axis, value, now),
        }
    }

    fn button(&mut self, button: Button, pressed: bool, now: Instant) -> Vec<Action> {
        let dir = match button {
            Button::DPadUp => Some(Direction::Up),
            Button::DPadDown => Some(Direction::Down),
            Button::DPadLeft => Some(Direction::Left),
            Button::DPadRight => Some(Direction::Right),
            _ => None,
        };
        if let Some(dir) = dir {
            if self.owner == InputOwner::App {
                return Vec::new();
            }
            self.dpad[index(dir)] = pressed;
            if pressed {
                self.dpad_last = Some(dir);
            } else if self.dpad_last == Some(dir) {
                self.dpad_last = Direction::ALL.into_iter().find(|d| self.dpad[index(*d)]);
            }
            return self.sync_direction(now);
        }

        if self.holds_on && self.owner == InputOwner::Launcher && self.holds.rule(button).is_some() {
            return self.held_button(button, pressed, now);
        }
        if !pressed {
            return Vec::new();
        }
        match (self.owner, self.map.action_for(button)) {
            (_, Some(Action::MainMenu)) => vec![Action::MainMenu],
            (InputOwner::Launcher, Some(action)) => vec![action],
            _ => Vec::new(),
        }
    }

    /// A button with a hold rule: nothing on press but the ring starting; a tap on release does the
    /// button's ordinary action; a completed hold does nothing more.
    fn held_button(&mut self, button: Button, pressed: bool, now: Instant) -> Vec<Action> {
        if pressed {
            return if self.holds.press(button, now) { vec![Action::Hold(button, HoldPhase::Started)] } else { Vec::new() };
        }
        match self.holds.release(button) {
            Released::Tap => {
                let mut out = vec![Action::Hold(button, HoldPhase::Cancelled)];
                out.extend(self.map.action_for(button));
                out
            }
            Released::AfterHold | Released::Nothing => Vec::new(),
        }
    }

    fn axis(&mut self, axis: Axis, value: f32, now: Instant) -> Vec<Action> {
        match axis {
            Axis::LeftX => self.stick.0 = value,
            Axis::LeftY => self.stick.1 = value,
            // The right stick pages: handled as discrete steps, not as focus movement.
            Axis::RightX | Axis::RightY => return Vec::new(),
        }
        if self.owner == InputOwner::App {
            return Vec::new();
        }
        let (x, y) = self.stick;
        let magnitude = x.abs().max(y.abs());
        let dominant = if x.abs() >= y.abs() {
            if x >= 0. { Direction::Right } else { Direction::Left }
        } else if y >= 0. {
            Direction::Up
        } else {
            Direction::Down
        };
        self.stick_dir = match self.stick_dir {
            _ if magnitude >= self.config.stick_enter => Some(dominant),
            Some(current) if magnitude >= self.config.stick_exit => Some(current),
            _ => None,
        };
        self.sync_direction(now)
    }

    /// The direction that should currently be active: the D-pad wins over the stick.
    fn effective(&self) -> Option<Direction> {
        self.dpad_last.or(self.stick_dir)
    }

    fn sync_direction(&mut self, now: Instant) -> Vec<Action> {
        match (self.effective(), self.repeating) {
            (None, _) => {
                self.repeating = None;
                Vec::new()
            }
            (Some(dir), Some((current, _))) if dir == current => Vec::new(),
            (Some(dir), _) => {
                self.repeating = Some((dir, now + self.config.repeat_delay));
                vec![Action::Navigate(dir)]
            }
        }
    }

    pub fn tick(&mut self, now: Instant) -> Vec<Action> {
        let mut out: Vec<Action> = self.holds.tick(now).map(|b| Action::Hold(b, HoldPhase::Completed)).into_iter().collect();
        let Some((dir, due)) = self.repeating else {
            return out;
        };
        if now < due {
            return out;
        }
        // Emit once per tick even after a long stall, so a hitch never causes a burst of moves.
        self.repeating = Some((dir, now + self.config.repeat_interval));
        out.push(Action::Navigate(dir));
        out
    }
}

#[cfg(test)]
mod tests;
