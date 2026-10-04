//! Launch settings a game may support, and how a choice becomes a command line or a config edit.
//!
//! The model in one paragraph: the launcher has a fixed catalog of [`SettingKey`]s (window mode,
//! resolution, upscaling ...). A game declares [`Capabilities`]: which keys it honors and, for each,
//! how to apply a value ([`Output`]: a command-line argument, an environment variable, or an edit
//! to one of the game's config files). The user has [`SettingsLayer`]s of values: global defaults
//! and per-game overrides. [`effective`] merges them for one game and one display, and [`plan`]
//! turns the result into a [`LaunchPlan`]. A key the game does not declare is never shown and never
//! applied: a setting that is displayed and not applied is worse than one that does not exist.
//!
//! Full rules: `docs/specs/game-settings.md`.
//!
//! * `key`, `value`: the catalog of keys and the typed values
//! * `capabilities`: what a game declares; `environment`: what the display offers
//! * `kinds`: the shape of each key for a given game and display; `narrow`: a game's constraint on it
//! * `layer`: stored user choices; `adapt`: fitting a stored value to a game; `options`: every value a
//!   picker offers, with its label
//! * `resolve`: layering the choices (`supported`, `effective`) and the entry point `plan`
//! * `build`, `render`: running a binding's outputs, filling templates, typing config values
//! * `plan`: the launch plan types and `apply_config`; `config_edit/`: JSON, TOML, INI and key-value
//!   editors (one file each) and the atomic write
//! * `testing`: builders for the unit tests
mod adapt;
mod build;
mod capabilities;
mod config_edit;
mod environment;
mod key;
mod kinds;
mod layer;
mod narrow;
mod options;
mod plan;
mod render;
mod resolve;
#[cfg(test)]
mod testing;
mod value;

pub use adapt::adapt;
pub use capabilities::{Base, Binding, Capabilities, ConfigFormat, ConfigPath, Constraint, Output, Target, ValueType};
pub use config_edit::{ConfigEditError, apply_edits, write_atomic};
pub use environment::{DisplayEnvironment, DisplayServer, Monitor};
pub use key::{Group, SettingKey};
pub use kinds::standard_kind;
pub use layer::SettingsLayer;
pub use options::{label, options};
pub use plan::{Bases, ConfigFileEdit, ConfigValue, KeyEdit, LaunchPlan};
pub use resolve::{Effective, SettingSpec, Source, all_specs, effective, plan, supported};
pub use value::{Choice, SettingValue, Size, ValueKind};
