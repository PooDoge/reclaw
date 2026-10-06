//! What Reclaw is doing in the background: installs, updates and mod downloads, and what to show
//! about them. Pure data and rules, so every interface and every window shows the same thing and the
//! behavior is tested without a window.
//!
//! * `types`: one [`Activity`] and its parts; `board`: [`ActivityBoard`], the list, changed only by [`ActivityEvent`]s
//! * `indicator`: what a card or a row shows for a game, and the desktop sidebar's entries
//! * `failure`: what a "Failed" indicator says on hover ([`FailureHint`]) and opens on press ([`FailureReport`], with the job's log)
//! * `format`: sizes, speeds and time left as text
//!
//! The host (the code that really downloads and builds) sends events; the UI never creates or
//! changes an activity. A finished update stays on the board until the app restarts, so the user can
//! see that their game updated and read what changed.
mod board;
mod failure;
mod format;
mod indicator;
mod types;

#[cfg(test)]
mod tests;

pub use board::{ActivityBoard, ActivityEvent, Change, Counts};
pub use failure::{
    EARLIER_FAILURE, FailureHint, FailureReport, HINT_CHARS, LineTone, hint_for_activity, hint_for_game, latest_failure, line_tone, report,
    short_reason,
};
pub use format::{format_bytes, format_eta, format_rate};
pub use indicator::{Indicator, IndicatorKind, SidebarEntry, SidebarKind, indicator_for, sidebar_entries};
pub use types::{Activity, ActivityId, Changelog, Kind, Outcome, Stage};
