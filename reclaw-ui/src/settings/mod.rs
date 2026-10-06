//! Settings as data, for both interfaces: the schemas Reclaw ships (`builders`, `schema`), the values
//! users edit (`values`), how those values are stored in the settings file (`persist`), what a launch
//! row shows and offers (`launch`), where new installs go (`location`), and where
//! each row sits on a page (`geometry`). Nothing here knows about Freya.
//!
//! Adding a setting is adding a [`Row`] in `builders`; it is shown, saved and restored without any
//! other change.
pub mod builders;
pub mod geometry;
pub mod launch;
pub mod location;
pub mod persist;
pub mod schema;
pub mod values;

pub use builders::*;
pub use geometry::{RowSlot, section_slots};
pub use launch::{LaunchContext, LaunchControl, LaunchOption};
pub use location::{FALLBACK_LOCATION, KEY_DEFAULT_LOCATION, default_install_location, resolve_install_location};
pub use schema::{CredentialPart, GlobalAction, Group, Row, RowAction, RowKind, Schema, Section, TextField};
pub use values::{SettingChange, SettingValue, SettingsTarget, SettingsValues};
