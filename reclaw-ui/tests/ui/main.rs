//! Every UI integration test in one binary. Each `tests/*.rs` file would link its own copy of the
//! UI toolkit (about a gigabyte in a debug build); one binary links it once.
//!
//! Run one file with `cargo test -p reclaw-ui --test ui deck_input::`.
//!
//! * `common`: the headless harness (`Mount`, `Session`) every test uses
//! * `deck_*`: Deck mode (input, lifecycle with real processes, notifications and holds, surfaces, snapshots)
//! * `desktop_*`, `nav_stage`: the desktop interface and the router
//! * `media`: artwork and READMEs against a fake internet
//! * `systems`: the system badge, the System filter and Sort in the Library, shelves per system in Deck mode
//! * `window_chrome`: the custom title bar and the Screen settings
//! * `shell_modes`: switching interface; `tokens_in_sync`: the design tokens against the Rust constants
mod common;
mod deck_input;
mod deck_lifecycle;
mod deck_notices;
mod deck_routes;
mod deck_snapshots;
mod deck_surfaces;
mod desktop_pages;
mod desktop_settings;
mod desktop_snapshots;
mod desktop_surfaces;
mod media;
mod nav_stage;
mod shell_modes;
mod systems;
mod tokens_in_sync;
mod window_chrome;
