use std::{
    fs,
    io::ErrorKind,
    path::{Component, Path, PathBuf},
};

use super::{
    capabilities::{Base, ConfigFormat, ConfigPath},
    config_edit::{ConfigEditError, apply_edits, write_atomic},
};

/// Everything the supervisor needs to start a game with the user's settings applied.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct LaunchPlan {
    /// Appended after the game's own arguments, in catalog order.
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    /// Edits to run before the game starts. Grouped per file so each file is read and written once.
    pub config_edits: Vec<ConfigFileEdit>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ConfigFileEdit {
    pub file: ConfigPath,
    pub format: ConfigFormat,
    pub edits: Vec<KeyEdit>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct KeyEdit {
    /// Dotted path inside the file (`Graphics.Width`); `section.key` for INI; the key for key-value.
    pub path: String,
    pub value: ConfigValue,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ConfigValue {
    Bool(bool),
    Int(i64),
    Float(f64),
    Text(String),
}

/// The directories a host resolved for one install.
#[derive(Clone, PartialEq, Debug)]
pub struct Bases {
    pub install: PathBuf,
    pub config: PathBuf,
    pub data: PathBuf,
}

impl Bases {
    /// The absolute path of `path`, refusing anything that would leave its base (`..`, absolute).
    pub fn resolve(&self, path: &ConfigPath) -> Result<PathBuf, ConfigEditError> {
        let relative = path.relative.as_str();
        if relative.trim().is_empty() {
            return Err(ConfigEditError::bad_path(relative, "the path is empty"));
        }
        let escapes = || ConfigEditError::EscapesBase(relative.to_string());
        // A catalog may be written on Windows: a backslash separates here too, and a drive is absolute.
        let drive = relative.as_bytes().get(..2).is_some_and(|b| b[0].is_ascii_alphabetic() && b[1] == b':');
        if relative.starts_with(['/', '\\']) || drive || Path::new(relative).is_absolute() {
            return Err(escapes());
        }
        let mut resolved = self.base(path.base).clone();
        let mut named = false;
        for part in relative.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".") {
            // Exactly one plain name: not `..`, not a root or prefix on any platform.
            if !matches!(Path::new(part).components().collect::<Vec<_>>().as_slice(), [Component::Normal(_)]) {
                return Err(escapes());
            }
            resolved.push(part);
            named = true;
        }
        if !named {
            return Err(ConfigEditError::bad_path(relative, "the path names no file"));
        }
        Ok(resolved)
    }

    pub fn base(&self, base: Base) -> &PathBuf {
        match base {
            Base::Install => &self.install,
            Base::Config => &self.config,
            Base::Data => &self.data,
        }
    }
}

impl LaunchPlan {
    pub fn is_empty(&self) -> bool {
        self.args.is_empty() && self.env.is_empty() && self.config_edits.is_empty()
    }

    /// Run every config edit, writing each file atomically. Returns the files written. Stops at the
    /// first error and leaves that file as it was.
    pub fn apply_config(&self, bases: &Bases) -> Result<Vec<PathBuf>, ConfigEditError> {
        // Work out every file first, so a file that cannot be edited stops the run before any is written.
        let mut pending = Vec::with_capacity(self.config_edits.len());
        for item in &self.config_edits {
            let path = bases.resolve(&item.file)?;
            let current = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) if e.kind() == ErrorKind::NotFound => String::new(),
                Err(source) => return Err(ConfigEditError::io(&path, source)),
            };
            let updated = apply_edits(item.format, &current, &item.edits).map_err(|e| e.in_file(&path))?;
            pending.push((path, updated));
        }
        let mut written = Vec::with_capacity(pending.len());
        for (path, text) in pending {
            write_atomic(&path, &text)?;
            written.push(path);
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests;
