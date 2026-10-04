use crate::controller::{ControllerKind, GlyphFace};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    pub const ALL: [Direction; 4] = [Self::Up, Self::Down, Self::Left, Self::Right];
}

/// Physical buttons, named by position (South is the bottom face button) so mapping does not
/// depend on a vendor's labels.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Button {
    South,
    East,
    West,
    North,
    LeftBumper,
    RightBumper,
    LeftTrigger,
    RightTrigger,
    Select,
    Start,
    Guide,
    LeftStick,
    RightStick,
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Axis {
    LeftX,
    LeftY,
    RightX,
    RightY,
}

/// Hardware-independent event. Axis values are -1.0..=1.0 with **+Y up** and **+X right**.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum RawEvent {
    Button {
        button: Button,
        pressed: bool,
    },
    Axis {
        axis: Axis,
        value: f32,
    },
    /// The active controller went away: drop held directions so nothing repeats forever.
    Disconnected,
}

/// What the UI reacts to. Names say what the action means, not which button produced it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Action {
    Navigate(Direction),
    Confirm,
    Back,
    /// Secondary verb of the focused item (Manage, Options).
    Secondary,
    /// Tertiary verb (Search).
    Tertiary,
    PrevSection,
    NextSection,
    PageUp,
    PageDown,
    /// Left slide-in menu (the Steam-button equivalent).
    MainMenu,
    /// Right slide-in panel: running app, controller, downloads.
    QuickAccess,
    /// Context menu of the focused item.
    Options,
    /// A button with a hold rule changed phase (see `InputMapper::set_hold_rules`). Only the mapper
    /// produces it.
    Hold(Button, crate::hold::HoldPhase),
}

/// Button to action table. Defaults follow Steam's Big Picture conventions; every entry is
/// replaceable.
#[derive(Clone, PartialEq, Debug)]
pub struct ActionMap {
    entries: Vec<(Button, Action)>,
}

impl Default for ActionMap {
    fn default() -> Self {
        use Button::*;
        Self {
            entries: vec![
                (South, Action::Confirm),
                (East, Action::Back),
                (West, Action::Secondary),
                (North, Action::Tertiary),
                (LeftBumper, Action::PrevSection),
                (RightBumper, Action::NextSection),
                (LeftTrigger, Action::PageUp),
                (RightTrigger, Action::PageDown),
                (Guide, Action::MainMenu),
                (Select, Action::QuickAccess),
                (Start, Action::Options),
            ],
        }
    }
}

impl ActionMap {
    /// Nintendo-style layout: Confirm on East, Back on South.
    pub fn swapped_confirm_back(mut self) -> Self {
        for (button, action) in &mut self.entries {
            match (*button, *action) {
                (Button::South, Action::Confirm) => *button = Button::East,
                (Button::East, Action::Back) => *button = Button::South,
                _ => {}
            }
        }
        self
    }

    pub fn rebind(&mut self, button: Button, action: Action) {
        self.entries.retain(|(b, a)| *b != button && *a != action);
        self.entries.push((button, action));
    }

    /// Remove a binding, e.g. Guide when Steam owns it.
    pub fn unbind(&mut self, button: Button) {
        self.entries.retain(|(b, _)| *b != button);
    }

    pub fn action_for(&self, button: Button) -> Option<Action> {
        self.entries.iter().find(|(b, _)| *b == button).map(|(_, a)| *a)
    }

    pub fn button_for(&self, action: Action) -> Option<Button> {
        self.entries.iter().find(|(_, a)| *a == action).map(|(b, _)| *b)
    }

    /// The glyph to print in a hint bar for `action` on this kind of controller.
    pub fn glyph(&self, action: Action, kind: ControllerKind) -> Option<GlyphFace> {
        self.button_for(action).map(|button| kind.glyph(button))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_big_picture() {
        let map = ActionMap::default();
        assert_eq!(map.action_for(Button::South), Some(Action::Confirm));
        assert_eq!(map.action_for(Button::East), Some(Action::Back));
        assert_eq!(map.action_for(Button::Guide), Some(Action::MainMenu));
    }

    #[test]
    fn swap_exchanges_confirm_and_back_only() {
        let map = ActionMap::default().swapped_confirm_back();
        assert_eq!(map.action_for(Button::East), Some(Action::Confirm));
        assert_eq!(map.action_for(Button::South), Some(Action::Back));
        assert_eq!(map.action_for(Button::West), Some(Action::Secondary));
    }

    #[test]
    fn unbind_removes_only_that_button() {
        let mut map = ActionMap::default();
        map.unbind(Button::Guide);
        assert_eq!(map.action_for(Button::Guide), None);
        assert_eq!(map.button_for(Action::MainMenu), None);
        assert_eq!(map.action_for(Button::South), Some(Action::Confirm));
    }

    #[test]
    fn rebind_removes_both_old_bindings() {
        let mut map = ActionMap::default();
        map.rebind(Button::North, Action::Confirm);
        assert_eq!(map.action_for(Button::South), None);
        assert_eq!(map.action_for(Button::North), Some(Action::Confirm));
        assert_eq!(map.button_for(Action::Tertiary), None);
    }
}
