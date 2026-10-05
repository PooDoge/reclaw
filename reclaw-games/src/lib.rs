//! Everything the launcher knows about a game that is not a window: what the project is
//! ([`project`]), which launch settings the game supports and how to apply them ([`settings`]).
//! No UI types, no I/O except the config-file editors, so it is tested without a window.
//!
//! * `platform`: the systems games were recompiled from, how they are named, grouped and ordered
//! * `project`: catalog metadata, art, media, releases, requirements
//! * `settings`: the setting keys, a game's capabilities, layered values, the launch plan
//! * `version`: telling release tags apart (`v1.4.2` is `1.4.2`; `1.4.2-beta` is not `1.4.2`), shared by the installer and the screens
//! * `fixtures` (tests and the `fixtures` feature): invented projects that exercise every kind of launch setting
#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
pub mod platform;
pub mod project;
pub mod settings;
pub mod version;
