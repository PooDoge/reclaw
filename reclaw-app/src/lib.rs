//! The launcher program, minus its windows: what answers the screens' requests.
//!
//! * `host`: [`Host`], which loads the catalog in the background, keeps the library file, and answers `Effect`s
//! * `browse`: opening a link in the system browser, https only
//!
//! `main.rs` is the thin part that starts the window and wires the gamepad.
pub mod browse;
pub mod host;

pub use host::{Host, Initial, Sink};
