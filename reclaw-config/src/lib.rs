//! What Reclaw remembers between runs: the user's settings, favorites, launch defaults and window
//! placement, in one human-editable TOML file. No UI types and no async runtime, so it is tested
//! with a temporary directory.
//!
//! * `prefs`: the [`Preferences`] data and its TOML form; `migrate`: reading older or newer files
//! * `paths`: where the file lives on each OS; `file`: tolerant load and atomic save
//! * `writer`: [`PrefsWriter`], a background thread that coalesces rapid changes into one write
//!
//! The rules: loading never fails (a damaged file is set aside as `.bad` and defaults are used), a
//! file written by a newer Reclaw is read but never overwritten, and a save is a temporary file plus
//! a rename, so a crash leaves the old file or the new one, never half of one.
mod error;
mod file;
mod migrate;
mod paths;
mod prefs;
mod writer;

pub use error::PrefsError;
pub use file::{Loaded, PrefsFile};
pub use paths::AppDirs;
pub use prefs::{LaunchPrefs, PrefValue, Preferences, WindowPrefs};
pub use writer::PrefsWriter;
