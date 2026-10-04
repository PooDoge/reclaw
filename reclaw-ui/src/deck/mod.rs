//! Deck mode: the controller-first shell.
//!
//! * `state` (with `settings`, `launch`, `layout`) is pure and tested without a window;
//! * `widgets` are single components; `pages` fill the body between the tabs and the hint bar;
//! * `app` is the root that wires input, state and rendering together.
mod app;
mod frame;
pub mod layout;
mod pages;
pub mod routes;
pub use crate::settings;
pub mod state;
mod widgets;

pub use crate::launch::{LaunchVerb, failure_message, launch_verb, shows_stop_pair};
pub use app::{ActionFeed, DeckApp, TextBoxes};
pub use frame::DeckFrame;
pub use pages::{DownloadsPage, EmptyPage, GamePage, HomePage, InstallBody, SettingsBody, install_footer};
pub use settings::{SettingChange, SettingValue, SettingsTarget, SettingsValues, TextField};
pub use state::{
    BANNER_BLOCK, BANNER_H, ConfirmKind, DeckState, DeckView, Effect, InstallDraft, LastInput, MAIN_MENU, MainMenuEntry, MenuAction,
    MenuPurpose, ModePref, Overlay, SHELF_H, SHELF_TITLE_BLOCK, Screen, Section, ShelfSpec, TWO_PANE_MIN_W, ids, shelf_top, shelves,
    tile_rect,
};
pub use widgets::*;
