//! The state every window and both interfaces share: the library, background activity,
//! notifications, the user's settings, and what the device reports. One [`AppState`] in one
//! `freya-radio` station, changed only by [`AppAction`]s.
//!
//! * `state`: [`AppState`], plain data, and how it becomes the settings file and back
//! * `action`: [`AppAction`], every way the state can change; `reduce`: what each does (pure, tested)
//! * `channel`: [`AppChannel`], who is told when something changed
//! * `handle`: [`Store`], the `Copy` handle components and handlers hold; `feed`: [`StoreFeed`] for other threads
//! * `hooks`: `use_games()`, `use_settings()` ...: read one part and re-render only when it changes
//! * `from_effect`: the UI-owned [`Effect`](crate::effect::Effect)s, as actions
//!
//! # Which kind of state goes where
//!
//! * **Shared by every window, survives a mode switch, or comes from outside the UI** (games, downloads,
//!   settings, notifications, the pad): here, in the store.
//! * **One window's business** (which page, the search text, an open dialog, focus in Deck mode):
//!   `use_state` in the component that owns it, or context provided by the frame (`Nav`, `DesktopUi`,
//!   `ShellCtx`).
//! * **Fetched, cached, can be fetched again** (README text, release lists): `freya-query`, or the
//!   image cache. Never in the store.
//!
//! # Adding a piece of state
//!
//! 1. A field on [`AppState`] (and, if it must survive a restart, a line in `to_prefs`/`from_prefs`).
//! 2. An [`AppChannel`] variant if readers should not redraw for unrelated changes.
//! 3. An [`AppAction`] variant and its arm in `reduce`, returning the channels it touched. Add a test.
//! 4. A `use_...()` hook. Components read with the hook and never reach into the station.
//!
//! Threads that are not the UI's (the supervisor, a downloader, the gamepad) cannot touch the store:
//! they send [`AppAction`]s through a [`StoreFeed`], and one task on the UI thread dispatches them.
mod action;
mod changes;
mod channel;
mod feed;
mod from_effect;
mod handle;
mod hooks;
mod reduce;
mod state;

#[cfg(test)]
mod tests;

pub use action::AppAction;
pub use channel::AppChannel;
pub use feed::{StoreFeed, StoreInbox, feed};
pub use from_effect::ui_action;
pub use handle::Store;
pub use hooks::*;
pub use state::{AppState, Mailbox};
