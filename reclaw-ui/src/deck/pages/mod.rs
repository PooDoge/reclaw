//! Full pages of Deck mode: what fills the area between the section tabs and the hint bar.
mod downloads;
mod empty;
mod game;
mod home;
mod install;
mod settings;

pub use downloads::DownloadsPage;
pub use empty::EmptyPage;
pub use game::GamePage;
pub use home::HomePage;
pub use install::{InstallBody, install_footer};
pub use settings::SettingsBody;
