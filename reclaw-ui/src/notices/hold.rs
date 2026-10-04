use std::time::Duration;

use reclaw_input::{Button, HoldRule};

/// Hold X to open the notification's details.
pub const DETAILS_BUTTON: Button = Button::West;
/// Hold Y to dismiss every notification.
pub const DISMISS_ALL_BUTTON: Button = Button::North;

/// How long each is held. Long enough that nobody triggers it while pressing the button for its
/// ordinary job (X is Options, Y is Search), short enough not to feel like a chore.
pub const DETAILS_AFTER: Duration = Duration::from_millis(900);
pub const DISMISS_ALL_AFTER: Duration = Duration::from_millis(1200);

/// What a completed hold asks for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HoldAction {
    Details,
    DismissAll,
}

impl HoldAction {
    /// The action for a button whose hold completed, if it is one of ours.
    pub fn of(button: Button) -> Option<Self> {
        match button {
            DETAILS_BUTTON => Some(Self::Details),
            DISMISS_ALL_BUTTON => Some(Self::DismissAll),
            _ => None,
        }
    }

    pub fn button(self) -> Button {
        match self {
            Self::Details => DETAILS_BUTTON,
            Self::DismissAll => DISMISS_ALL_BUTTON,
        }
    }

    pub fn after(self) -> Duration {
        match self {
            Self::Details => DETAILS_AFTER,
            Self::DismissAll => DISMISS_ALL_AFTER,
        }
    }
}

/// The rules to give the gamepad reader: which buttons can be held while a notification shows.
pub fn hold_rules() -> Vec<HoldRule> {
    [HoldAction::Details, HoldAction::DismissAll].into_iter().map(|a| HoldRule { button: a.button(), after: a.after() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_hold_button_maps_to_its_action_and_back() {
        for action in [HoldAction::Details, HoldAction::DismissAll] {
            assert_eq!(HoldAction::of(action.button()), Some(action));
        }
        assert_eq!(HoldAction::of(Button::South), None);
    }

    #[test]
    fn dismissing_everything_takes_longer_than_looking_closer() {
        assert!(DISMISS_ALL_AFTER > DETAILS_AFTER, "the destructive one needs the longer hold");
        assert_eq!(hold_rules().len(), 2);
    }
}
