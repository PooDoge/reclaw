//! Messages for the user about things that happened in the background: an update is ready, a game
//! finished updating, a download failed. Pure data and rules; the Deck interface shows the newest as
//! a toast with press-and-hold controls, the desktop as a badge and a list.
//!
//! * `types`: [`Notice`] and its kinds; `queue`: [`Notices`], the list with its rules
//!   (a repeat replaces, the newest is shown, dismiss one or all)
//! * `hold`: which buttons are held for what (the gesture itself is `reclaw_input::HoldTracker`)
mod hold;
mod queue;
mod types;

#[cfg(test)]
mod tests;

pub use hold::{DETAILS_AFTER, DETAILS_BUTTON, DISMISS_ALL_AFTER, DISMISS_ALL_BUTTON, HoldAction, hold_rules};
pub use queue::Notices;
pub use types::{Notice, NoticeId, NoticeKind};
