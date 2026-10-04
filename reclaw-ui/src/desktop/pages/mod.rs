//! The desktop version of each routed page. `crate::pages` picks these or the Deck ones by interface.
//!
//! * `common`: the page header, scroll body, cards and capsule grid the pages share
//! * `library`: the Library at all three layout classes; `catalog`, `game`, `mods`, `mod_detail`,
//!   `downloads`, `install`: one page each (`game/` has a file per section of the page)
//! * `stubs`: stand-ins for the pages not built yet (the settings pages)
mod catalog;
pub mod common;
mod downloads;
mod game;
mod install;
pub mod library;
mod mod_detail;
mod mods;
mod stubs;

pub use catalog::CatalogPage;
pub use downloads::DownloadsPage;
pub use game::GamePage;
pub use install::InstallPage;
pub use library::LibraryPage;
pub use mod_detail::ModDetailPage;
pub use mods::ModsPage;
pub use stubs::*;
