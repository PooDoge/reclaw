//! Every UI integration test in one binary. Each `tests/*.rs` file would link its own copy of the
//! UI toolkit (about a gigabyte in a debug build); one binary links it once.
//!
//! Run one file with `cargo test -p reclaw-ui --test ui deck_input::`.
//!
//! * `common`: the headless harness (`Mount`, `Session`) every test uses
//! * `deck_*`: Deck mode (input, lifecycle with real processes, notifications and holds, surfaces, snapshots)
//! * `credentials`: the Network and Diagnostics settings, and that a pasted token leaves the page once, as a secret
//! * `desktop_*`, `nav_stage`: the desktop interface and the router (`desktop_failures`: the tooltip and log behind "Failed";
//!   `desktop_search`: the search button, each tab's results, the Mods tab's shelves)
//! * `recents`: the Recent menu (desktop) and the Quick access list (Deck)
//! * `media`: artwork and READMEs against a fake internet; `banner`: what the game page shows when the catalog has no banner
//! * `real_data`: data arriving after the first frame, and (with `QUIVER_CATALOG_DIR`) the real catalog on both interfaces
//! * `systems`: the system badge, the System filter and Sort in the Library, shelves per system in Deck mode
//! * `window_chrome`: the custom title bar and the Screen settings
//! * `shell_modes`: switching interface; `tokens_in_sync`: the design tokens against the Rust constants
mod banner;
mod common;
mod credentials;
mod deck_input;
mod deck_lifecycle;
mod deck_notices;
mod deck_routes;
mod deck_snapshots;
mod deck_surfaces;
mod deck_theme;
mod desktop_failures;
mod desktop_pages;
mod desktop_search;
mod desktop_settings;
mod desktop_snapshots;
mod desktop_surfaces;
mod media;
mod nav_stage;
mod real_data;
mod recents;
mod shell_modes;
mod systems;
mod tokens_in_sync;
mod window_chrome;
