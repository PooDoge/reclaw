//! Every UI integration test in one binary. Each `tests/*.rs` file would link its own copy of the
//! UI toolkit (about a gigabyte in a debug build); one binary links it once.
//!
//! Run one file with `cargo test -p reclaw-ui --test ui deck_input::`.
//!
//! * `common`: the headless harness (`Mount`, `Session`) every test uses
//! * `deck_*`: Deck mode (input, lifecycle with real processes, surfaces, snapshots)
//! * `desktop_*`, `nav_stage`: the desktop interface and the router
//! * `shell_modes`: switching interface; `tokens_in_sync`: the design tokens against the Rust constants
mod common;
mod deck_input;
mod deck_lifecycle;
mod deck_routes;
mod deck_snapshots;
mod deck_surfaces;
mod desktop_pages;
mod desktop_settings;
mod desktop_snapshots;
mod desktop_surfaces;
mod nav_stage;
mod shell_modes;
mod tokens_in_sync;
