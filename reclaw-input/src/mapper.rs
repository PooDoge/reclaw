use std::time::{Duration, Instant};

use crate::action::{Action, ActionMap, Axis, Button, Direction, RawEvent};

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

    fn reset(&mut self) {
        self.dpad = [false; 4];
        self.dpad_last = None;
        self.stick = (0., 0.);
        self.stick_dir = None;
        self.repeating = None;
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

        if !pressed {
            return Vec::new();
        }
        match (self.owner, self.map.action_for(button)) {
            (_, Some(Action::MainMenu)) => vec![Action::MainMenu],
            (InputOwner::Launcher, Some(action)) => vec![action],
            _ => Vec::new(),
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
        let Some((dir, due)) = self.repeating else {
            return Vec::new();
        };
        if now < due {
            return Vec::new();
        }
        // Emit once per tick even after a long stall, so a hitch never causes a burst of moves.
        self.repeating = Some((dir, now + self.config.repeat_interval));
        vec![Action::Navigate(dir)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> (InputMapper, Instant) {
        (InputMapper::new(ActionMap::default(), MapperConfig::default()), Instant::now())
    }

    fn press(button: Button) -> RawEvent {
        RawEvent::Button { button, pressed: true }
    }

    fn release(button: Button) -> RawEvent {
        RawEvent::Button { button, pressed: false }
    }

    #[test]
    fn face_buttons_map_on_press_only() {
        let (mut m, t) = mapper();
        assert_eq!(m.handle(press(Button::South), t), vec![Action::Confirm]);
        assert!(m.handle(release(Button::South), t).is_empty());
    }

    #[test]
    fn dpad_moves_then_repeats_after_the_delay() {
        let (mut m, t) = mapper();
        assert_eq!(m.handle(press(Button::DPadRight), t), vec![Action::Navigate(Direction::Right)]);
        assert!(m.tick(t + Duration::from_millis(399)).is_empty());
        assert_eq!(m.tick(t + Duration::from_millis(400)), vec![Action::Navigate(Direction::Right)]);
        assert!(m.tick(t + Duration::from_millis(450)).is_empty());
        assert_eq!(m.tick(t + Duration::from_millis(510)), vec![Action::Navigate(Direction::Right)]);
    }

    #[test]
    fn releasing_stops_the_repeat() {
        let (mut m, t) = mapper();
        m.handle(press(Button::DPadDown), t);
        m.handle(release(Button::DPadDown), t + Duration::from_millis(100));
        assert!(m.tick(t + Duration::from_secs(2)).is_empty());
    }

    #[test]
    fn stall_does_not_burst() {
        let (mut m, t) = mapper();
        m.handle(press(Button::DPadUp), t);
        assert_eq!(m.tick(t + Duration::from_secs(5)).len(), 1);
    }

    #[test]
    fn newest_dpad_direction_wins_and_falls_back() {
        let (mut m, t) = mapper();
        m.handle(press(Button::DPadUp), t);
        assert_eq!(m.handle(press(Button::DPadLeft), t), vec![Action::Navigate(Direction::Left)]);
        // Letting go of Left falls back to the still-held Up.
        assert_eq!(m.handle(release(Button::DPadLeft), t), vec![Action::Navigate(Direction::Up)]);
    }

    #[test]
    fn stick_has_hysteresis() {
        let (mut m, t) = mapper();
        let x = |value| RawEvent::Axis { axis: Axis::LeftX, value };
        assert!(m.handle(x(0.5), t).is_empty(), "below enter threshold");
        assert_eq!(m.handle(x(0.7), t), vec![Action::Navigate(Direction::Right)]);
        assert!(m.handle(x(0.5), t).is_empty(), "between exit and enter keeps the direction, no new move");
        assert!(m.handle(x(0.3), t).is_empty(), "released");
        assert!(m.tick(t + Duration::from_secs(1)).is_empty(), "and no repeat after release");
        assert_eq!(m.handle(x(0.7), t), vec![Action::Navigate(Direction::Right)]);
    }

    #[test]
    fn stick_up_is_positive_y() {
        let (mut m, t) = mapper();
        let y = RawEvent::Axis { axis: Axis::LeftY, value: 0.9 };
        assert_eq!(m.handle(y, t), vec![Action::Navigate(Direction::Up)]);
    }

    #[test]
    fn app_ownership_swallows_everything_but_guide() {
        let (mut m, t) = mapper();
        m.set_owner(InputOwner::App);
        assert!(m.handle(press(Button::South), t).is_empty());
        assert!(m.handle(press(Button::DPadRight), t).is_empty());
        assert!(m.tick(t + Duration::from_secs(1)).is_empty());
        assert_eq!(m.handle(press(Button::Guide), t), vec![Action::MainMenu]);
    }

    #[test]
    fn switching_owner_clears_held_directions() {
        let (mut m, t) = mapper();
        m.handle(press(Button::DPadRight), t);
        m.set_owner(InputOwner::App);
        m.set_owner(InputOwner::Launcher);
        assert!(m.tick(t + Duration::from_secs(1)).is_empty(), "no phantom repeat after returning");
    }

    #[test]
    fn disconnect_clears_state() {
        let (mut m, t) = mapper();
        m.handle(press(Button::DPadRight), t);
        m.handle(RawEvent::Disconnected, t);
        assert!(m.tick(t + Duration::from_secs(1)).is_empty());
    }
}
