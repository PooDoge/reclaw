//! Editing a game's config file in place: one function per format, all pure, plus the atomic write.
use std::path::Path;

use super::{capabilities::ConfigFormat, plan::KeyEdit};

mod files;
mod ini;
mod json;
mod key_value;
mod lines;
mod toml;

pub use files::write_atomic;

#[derive(Debug, thiserror::Error)]
pub enum ConfigEditError {
    /// The file is not valid for its format. `apply_edits` names the format, `apply_config` the file.
    #[error("{path}: {message}")]
    Parse { path: String, message: String },
    #[error("path {0:?} would leave the game's directory")]
    EscapesBase(String),
    #[error("cannot set {path:?}: {message}")]
    BadPath { path: String, message: String },
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

impl ConfigEditError {
    pub(super) fn bad_path(path: &str, message: impl Into<String>) -> Self {
        Self::BadPath { path: path.to_string(), message: message.into() }
    }

    pub(super) fn parse(format: &str, message: impl ToString) -> Self {
        Self::Parse { path: format.to_string(), message: message.to_string() }
    }

    pub(super) fn io(path: &Path, source: std::io::Error) -> Self {
        Self::Io { path: path.display().to_string(), source }
    }

    /// A parse error that names the file instead of the format.
    pub(super) fn in_file(self, file: &Path) -> Self {
        match self {
            Self::Parse { message, .. } => Self::Parse { path: file.display().to_string(), message },
            other => other,
        }
    }
}

/// Return `text` with each edit applied, leaving everything else as it was (comments, order, spacing
/// where the format allows). Missing keys are created. An empty `text` is a new file.
pub fn apply_edits(format: ConfigFormat, text: &str, edits: &[KeyEdit]) -> Result<String, ConfigEditError> {
    match format {
        ConfigFormat::Json => json::apply(text, edits),
        ConfigFormat::Toml => toml::apply(text, edits),
        ConfigFormat::Ini => ini::apply(text, edits),
        ConfigFormat::KeyValue => key_value::apply(text, edits),
    }
}

#[cfg(test)]
mod tests;
