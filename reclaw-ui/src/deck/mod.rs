//! Deck mode: the controller-first shell. `state` and `launch` are pure and tested; the rest
//! renders them.
mod app;
mod backdrop;
mod banner;
mod focus;
mod glyph;
mod hint_bar;
pub mod launch;
mod launch_button;
pub mod layout;
mod pages;
mod panel;
mod shelf;
pub mod state;
mod tabs;
mod tile;

pub use launch::{LaunchVerb, failure_message, launch_verb, shows_stop_pair};
pub use state::{
    DeckState, DeckView, Effect, LastInput, MENU, MenuEntry, Overlay, Screen, Section, ids,
};

pub use app::{ActionFeed, DeckApp};
pub use backdrop::Backdrop;
pub use banner::{NowPlayingBanner, elapsed_label};
pub use focus::FocusFrame;
pub use glyph::ButtonGlyph;
pub use hint_bar::HintBar;
pub use launch_button::LaunchButton;
pub use pages::{DownloadsPage, EmptyPage, GamePage, HomePage};
pub use panel::{MainMenu, PanelSide, QuickAccess, SlidePanel};
pub use shelf::Shelf;
pub use tabs::SectionTabs;
pub use tile::DeckTile;
