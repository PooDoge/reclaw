use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum PrefsError {
    #[error("cannot read or write {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path} is not valid settings: {message}")]
    Parse { path: PathBuf, message: String },
    #[error("settings cannot be written as TOML: {0}")]
    Encode(String),
    #[error("{path} was written by a newer Reclaw (format {found}, this one reads {known}); it is left alone")]
    TooNew { path: PathBuf, found: u32, known: u32 },
}

impl PrefsError {
    pub(crate) fn io(path: &std::path::Path, source: std::io::Error) -> Self {
        Self::Io { path: path.to_path_buf(), source }
    }
}
