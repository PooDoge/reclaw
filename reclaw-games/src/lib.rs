//! Everything the launcher knows about a game that is not a window: what the project is
//! ([`project`]), which launch settings the game supports and how to apply them ([`settings`]).
//! No UI types, no I/O except the config-file editors, so it is tested without a window.
//!
//! * `project`: catalog metadata, art, media, releases, requirements
//! * `settings`: the setting keys, a game's capabilities, layered values, the launch plan
//! * `sample`: sample projects for the gallery and the tests
pub mod project;
pub mod sample;
pub mod settings;
