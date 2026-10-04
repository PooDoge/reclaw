//! Settings pages as data: the schemas Reclaw ships, the values users edit, and row geometry.
pub mod builders;
pub mod geometry;
pub mod schema;
pub mod values;

pub use builders::{KEY_INTERFACE_MODE, app_properties, global_settings};
pub use geometry::{RowSlot, section_slots};
pub use schema::{Group, Row, RowAction, RowKind, Schema, Section, TextField};
pub use values::{SettingChange, SettingValue, SettingsTarget, SettingsValues};
