//! The access tokens, in a file of their own. Not in the settings file: that one is the thing a person pastes into a bug report,
//! and a token pasted along with it is a token published.
//!
//! * The file is created readable by its owner only (`0600` on Unix; on Windows it sits in the user's own profile folder, which
//!   other accounts cannot read). A file found with looser permissions is tightened, and the user is told.
//! * A save is a private temporary file plus a rename, so a crash leaves the old file or the new one, and the token is never in a
//!   file other users could read, even for a moment.
//! * Loading never fails: a missing file is no tokens, a damaged one is set aside as `secrets.toml.bad` (the person may want to
//!   recover a token from it), and one that cannot be read at all is left alone and not overwritten.
//! * Every token read or written is registered with the log's redaction, so it cannot be written to a log in any form.
use std::{
    collections::BTreeMap,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use reclaw_log::{Secret, register_secret};
use serde::{Deserialize, Serialize};

const FORMAT: u32 = 1;

const HEADER: &str = "# Reclaw's access tokens. Keep this file private: anyone who can read it can use your tokens.\n\
# Remove a token in Settings (Network), or delete its line here.\n";

#[derive(Debug, thiserror::Error)]
pub enum SecretsError {
    #[error("cannot read or write {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("the tokens cannot be written as TOML: {0}")]
    Encode(String),
}

/// Tokens by name (`github`, `gitlab`). Printing one shows no part of any token.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Secrets(BTreeMap<String, Secret>);

impl Secrets {
    pub fn get(&self, name: &str) -> Option<&Secret> {
        self.0.get(name)
    }

    /// Set the token called `name`, or remove it with `None`.
    pub fn set(&mut self, name: &str, token: Option<Secret>) {
        match token {
            Some(token) => self.0.insert(name.to_string(), token),
            None => self.0.remove(name),
        };
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Serialize, Deserialize)]
struct Disk {
    version: u32,
    #[serde(default)]
    tokens: BTreeMap<String, String>,
}

/// What [`SecretsFile::load`] found.
#[derive(Debug)]
pub struct LoadedSecrets {
    pub secrets: Secrets,
    /// Something to tell the person once: the file was damaged, was readable by others, or could not be read.
    pub warning: Option<String>,
    /// The file could not be read, so it must not be overwritten (it may still hold tokens).
    pub read_only: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SecretsFile {
    path: PathBuf,
}

impl SecretsFile {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> LoadedSecrets {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return LoadedSecrets { secrets: Secrets::default(), warning: None, read_only: false };
            }
            Err(e) => {
                tracing::error!(path = %self.path.display(), error = %e, "the tokens file could not be read; it is left alone");
                return LoadedSecrets {
                    secrets: Secrets::default(),
                    warning: Some(format!(
                        "The tokens file {} could not be read ({e}), so no saved token is used. It was not changed.",
                        self.path.display()
                    )),
                    read_only: true,
                };
            }
        };
        let mut warning = self.tighten();
        match toml::from_str::<Disk>(&text) {
            Ok(disk) if disk.version <= FORMAT => {
                let mut secrets = Secrets::default();
                for (name, value) in disk.tokens {
                    let value = value.trim();
                    if value.is_empty() {
                        continue;
                    }
                    register_secret(value);
                    secrets.set(&name, Some(Secret::new(value)));
                }
                LoadedSecrets { secrets, warning, read_only: false }
            }
            Ok(disk) => {
                tracing::warn!(path = %self.path.display(), found = disk.version, known = FORMAT, "the tokens file is from a newer Reclaw; it is left alone");
                LoadedSecrets {
                    secrets: Secrets::default(),
                    warning: Some(format!(
                        "The tokens file was written by a newer Reclaw (format {}), so it is not used and not changed.",
                        disk.version
                    )),
                    read_only: true,
                }
            }
            Err(e) => {
                let aside = self.path.with_extension("toml.bad");
                let moved = fs::rename(&self.path, &aside);
                tracing::warn!(path = %self.path.display(), error = %e, moved = moved.is_ok(), "the tokens file is not valid and was set aside");
                let what = match moved {
                    Ok(()) => format!("The tokens file was not valid ({e}). It was kept as {} and no token is used.", aside.display()),
                    Err(rename) => {
                        format!("The tokens file was not valid ({e}) and could not be moved aside ({rename}); no token is used.")
                    }
                };
                warning = Some(warning.map_or(what.clone(), |w| format!("{w} {what}")));
                LoadedSecrets { secrets: Secrets::default(), warning, read_only: false }
            }
        }
    }

    /// Replace the file with `secrets`. With none left, the file is removed rather than left empty.
    pub fn save(&self, secrets: &Secrets) -> Result<(), SecretsError> {
        let io = |source| SecretsError::Io { path: self.path.clone(), source };
        if secrets.is_empty() {
            return match fs::remove_file(&self.path) {
                Ok(()) => Ok(()),
                Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
                Err(e) => Err(io(e)),
            };
        }
        let disk = Disk { version: FORMAT, tokens: secrets.0.iter().map(|(k, v)| (k.clone(), v.expose().to_string())).collect() };
        let body = toml::to_string(&disk).map_err(|e| SecretsError::Encode(e.to_string()))?;
        let parent = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::create_dir_all(parent).map_err(io)?;
        let temp = self.path.with_extension("toml.tmp");
        let written = (|| {
            // A leftover from a crash would make `create_new` fail forever.
            let _ = fs::remove_file(&temp);
            let mut file = private_file(&temp)?;
            file.write_all(HEADER.as_bytes())?;
            file.write_all(body.as_bytes())?;
            file.sync_all()?;
            fs::rename(&temp, &self.path)
        })();
        if let Err(e) = written {
            // Best effort: the error being returned is the one that matters.
            let _ = fs::remove_file(&temp);
            return Err(io(e));
        }
        for token in secrets.0.values() {
            register_secret(token.expose());
        }
        Ok(())
    }

    /// Remove access for group and others from a file that has it, and say so.
    #[cfg(unix)]
    fn tighten(&self) -> Option<String> {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&self.path).ok()?.permissions().mode();
        if mode & 0o077 == 0 {
            return None;
        }
        match fs::set_permissions(&self.path, fs::Permissions::from_mode(0o600)) {
            Ok(()) => {
                tracing::warn!(path = %self.path.display(), mode = format!("{:o}", mode & 0o777), "the tokens file could be read by others; made private");
                Some(format!("The tokens file could be read by other users (mode {:o}). It is private now.", mode & 0o777))
            }
            Err(e) => {
                tracing::error!(path = %self.path.display(), error = %e, "the tokens file could be read by others and could not be made private");
                Some(format!(
                    "The tokens file can be read by other users and could not be made private ({e}). Run: chmod 600 {}",
                    self.path.display()
                ))
            }
        }
    }

    #[cfg(not(unix))]
    fn tighten(&self) -> Option<String> {
        None
    }
}

/// A new file only its owner can read, from the first byte.
#[cfg(unix)]
fn private_file(path: &Path) -> std::io::Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)
}

#[cfg(not(unix))]
fn private_file(path: &Path) -> std::io::Result<fs::File> {
    fs::OpenOptions::new().write(true).create_new(true).open(path)
}

#[cfg(test)]
mod tests;
