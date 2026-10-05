//! The user's library file, `apps.json`, kept the way a person's list of games deserves: never replaced by something shorter
//! because of a read error, never half-written, with a copy of what it replaced, and not written by two copies of the program
//! at once.
//!
//! * A file that cannot be read **as a whole** is an error and is never overwritten (`load` fails, and the host keeps the library
//!   read-only until the person chooses `set_aside`).
//! * `save` writes a temporary file beside it, flushes it to disk and renames it over, so a crash leaves the old file or the new.
//! * Before replacing a file with different content, the old content is copied to `backups/apps-<hash>.json` (the same content is
//!   never copied twice; the newest few are kept).
//! * A lock file keeps two running copies from saving at the same moment.
use std::{
    fs::{self, File, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    time::SystemTime,
};

use reclaw_catalog::{AppEntry, library_to_string, parse_library};
use sha2::{Digest, Sha256};

/// How many backups are kept.
const KEEP_BACKUPS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("{path} could not be read: {detail}")]
    Unreadable { path: PathBuf, detail: String },
    #[error("{path} is not a valid library: {detail}")]
    Corrupt { path: PathBuf, detail: String },
    #[error("another copy of Reclaw is saving the library right now")]
    Locked,
    #[error("the library could not be saved to {path}: {detail}")]
    Write { path: PathBuf, detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryStore {
    path: PathBuf,
}

fn write_error(path: &Path, e: impl std::fmt::Display) -> LibraryError {
    LibraryError::Write { path: path.to_path_buf(), detail: e.to_string() }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

impl LibraryStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn sibling(&self, name: &str) -> PathBuf {
        self.path.with_file_name(name)
    }

    /// The library. A missing file is an empty library (the first run). Anything else that goes wrong is an error, so the
    /// caller cannot mistake "could not read" for "empty" and save over it.
    pub fn load(&self) -> Result<Vec<AppEntry>, LibraryError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(LibraryError::Unreadable { path: self.path.clone(), detail: e.to_string() }),
        };
        parse_library(&text).map_err(|e| LibraryError::Corrupt { path: self.path.clone(), detail: e.to_string() })
    }

    /// Replace the library with `apps`.
    pub fn save(&self, apps: &[AppEntry]) -> Result<(), LibraryError> {
        let parent = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
        fs::create_dir_all(parent).map_err(|e| write_error(&self.path, e))?;
        let lock = File::create(self.sibling("apps.json.lock")).map_err(|e| write_error(&self.path, e))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(LibraryError::Locked),
            Err(TryLockError::Error(e)) => return Err(write_error(&self.path, e)),
        }
        let text = library_to_string(apps);
        let existing = fs::read(&self.path).ok();
        if existing.as_deref() == Some(text.as_bytes()) {
            return Ok(());
        }
        if let Some(old) = &existing {
            self.back_up(old);
        }
        let temp = self.sibling(&format!("apps.json.tmp{}", std::process::id()));
        let written = (|| -> std::io::Result<()> {
            let mut file = File::create(&temp)?;
            file.write_all(text.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)
        })();
        if let Err(e) = written {
            // Best effort: the error that matters is the one being returned.
            let _ = fs::remove_file(&temp);
            return Err(write_error(&self.path, e));
        }
        // Make the rename itself durable. Not every platform can open a directory, so a failure here is not an error.
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
        drop(lock);
        Ok(())
    }

    /// Keep a copy of what is about to be replaced. A failed backup does not stop the save: losing the chance to undo is bad, but
    /// refusing to save the person's change is worse.
    fn back_up(&self, old: &[u8]) {
        let dir = self.sibling("backups");
        let name = format!("apps-{}.json", &hex(&Sha256::digest(old))[..16]);
        let target = dir.join(name);
        let result = (|| -> std::io::Result<()> {
            fs::create_dir_all(&dir)?;
            if !target.exists() {
                fs::write(&target, old)?;
            }
            self.prune(&dir)
        })();
        if let Err(e) = result {
            tracing::warn!(dir = %dir.display(), error = %e, "could not back up the library before saving; saving anyway");
        }
    }

    fn prune(&self, dir: &Path) -> std::io::Result<()> {
        let mut files: Vec<(SystemTime, PathBuf)> = fs::read_dir(dir)?
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| Some((e.metadata().ok()?.modified().ok()?, e.path())))
            .collect();
        files.sort_by_key(|(modified, _)| std::cmp::Reverse(*modified));
        for (_, path) in files.into_iter().skip(KEEP_BACKUPS) {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Move a library that cannot be read out of the way (to `apps.json.damaged-<hash>`) so a new one can be started, and say
    /// where it went. The person asks for this; the program never does it on its own.
    pub fn set_aside(&self) -> Result<PathBuf, LibraryError> {
        let bytes = fs::read(&self.path).map_err(|e| LibraryError::Unreadable { path: self.path.clone(), detail: e.to_string() })?;
        let target = self.sibling(&format!("apps.json.damaged-{}", &hex(&Sha256::digest(&bytes))[..12]));
        fs::rename(&self.path, &target).map_err(|e| write_error(&self.path, e))?;
        Ok(target)
    }
}

#[cfg(test)]
mod tests;
