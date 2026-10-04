//! Theme switching: one `ThemeKind` state drives both Freya's `Theme` (for built-in components)
//! and the full `Reclaw` token set (for everything the `ColorsSheet` has no slot for).
use freya::prelude::*;

pub use crate::tokens::{Reclaw, daylight, midnight, reclaw_daylight, reclaw_midnight};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeKind {
    #[default]
    Midnight,
    Daylight,
}

impl ThemeKind {
    pub fn tokens(self) -> Reclaw {
        match self {
            Self::Midnight => midnight(),
            Self::Daylight => daylight(),
        }
    }

    pub fn theme(self) -> Theme {
        match self {
            Self::Midnight => reclaw_midnight(),
            Self::Daylight => reclaw_daylight(),
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            Self::Midnight => Self::Daylight,
            Self::Daylight => Self::Midnight,
        }
    }
}

/// Call once in the app root. Returns the state so the root can offer a theme toggle.
pub fn use_init_reclaw(initial: ThemeKind) -> State<ThemeKind> {
    let kind = use_state(move || initial);
    let kind = use_provide_context(move || kind);
    let mut theme = use_init_theme(move || initial.theme());
    use_side_effect(move || {
        theme.set(kind().theme());
    });
    kind
}

/// Current tokens. Subscribes the calling component to theme changes.
pub fn use_reclaw() -> Reclaw {
    let kind = use_consume::<State<ThemeKind>>();
    kind().tokens()
}
