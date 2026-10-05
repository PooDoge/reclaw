//! Where each app was installed. The folder itself says what is installed (its `version.txt`); this remembers *where*, so an
//! app installed under one default location is still found after the default is changed.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::error::InstallError;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default = "one")]
    version: u32,
    #[serde(default)]
    apps: BTreeMap<String, Entry>,
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Entry {
    path: PathBuf,
}

/// The remembered folders, keyed by the app's identity key.
#[derive(Debug)]
pub struct InstallBook {
    file: PathBuf,
    apps: BTreeMap<String, Entry>,
}

impl InstallBook {
    /// Read the book. A file that cannot be read is kept as `<name>.bad` and the book starts empty: the folders on disk are still
    /// there, and each one is found again by its name in the default location.
    pub fn load(file: impl Into<PathBuf>) -> (Self, Option<String>) {
        let file = file.into();
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return (Self { file, apps: BTreeMap::new() }, None),
            Err(error) => {
                let problem = format!("{} could not be read: {error}", file.display());
                tracing::warn!(file = %file.display(), %error, "the install book could not be read");
                return (Self { file, apps: BTreeMap::new() }, Some(problem));
            }
        };
        match serde_json::from_str::<Stored>(&text) {
            Ok(stored) => (Self { file, apps: stored.apps }, None),
            Err(error) => {
                let aside = file.with_extension("json.bad");
                let moved = fs::rename(&file, &aside);
                tracing::warn!(file = %file.display(), %error, aside = %aside.display(), moved = moved.is_ok(), "the install book was not valid and was set aside");
                let problem = format!("{} was not valid ({error}); it was kept as {}", file.display(), aside.display());
                (Self { file, apps: BTreeMap::new() }, Some(problem))
            }
        }
    }

    pub fn path_of(&self, key: &str) -> Option<&Path> {
        self.apps.get(key).map(|e| e.path.as_path())
    }

    pub fn remember(&mut self, key: &str, path: &Path) -> Result<(), InstallError> {
        if self.apps.get(key).is_some_and(|e| e.path == path) {
            return Ok(());
        }
        self.apps.insert(key.to_string(), Entry { path: path.to_path_buf() });
        self.save()
    }

    pub fn forget(&mut self, key: &str) -> Result<(), InstallError> {
        if self.apps.remove(key).is_some() { self.save() } else { Ok(()) }
    }

    fn save(&self) -> Result<(), InstallError> {
        let text = serde_json::to_string_pretty(&Stored { version: 1, apps: self.apps.clone() })
            .map_err(|e| InstallError::BadAnswer(e.to_string()))?;
        if let Some(dir) = self.file.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir).map_err(|e| InstallError::io(format!("making {}", dir.display()), &e))?;
        }
        reclaw_net::cache::write_atomically(&self.file, text.as_bytes())
            .map_err(|e| InstallError::io(format!("saving {}", self.file.display()), &e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_remembered_survives_a_restart() {
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("data/installs.json");
        let (mut book, problem) = InstallBook::load(&file);
        assert!(problem.is_none() && book.path_of("github:o/r:Game").is_none());
        book.remember("github:o/r:Game", Path::new("/games/Game")).expect("saved");
        let (again, problem) = InstallBook::load(&file);
        assert!(problem.is_none());
        assert_eq!(again.path_of("github:o/r:Game"), Some(Path::new("/games/Game")));
    }

    #[test]
    fn forgetting_removes_it_for_good() {
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("installs.json");
        let (mut book, _) = InstallBook::load(&file);
        book.remember("k", Path::new("/a")).expect("saved");
        book.forget("k").expect("saved");
        book.forget("never-there").expect("nothing to do");
        assert!(InstallBook::load(&file).0.path_of("k").is_none());
    }

    #[test]
    fn a_book_that_is_not_json_is_set_aside_not_lost_and_not_fatal() {
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("installs.json");
        fs::write(&file, "{ not json").expect("write");
        let (book, problem) = InstallBook::load(&file);
        assert!(book.path_of("k").is_none());
        assert!(problem.is_some_and(|p| p.contains("installs.json.bad")));
        assert_eq!(fs::read_to_string(dir.path().join("installs.json.bad")).expect("kept"), "{ not json");
    }

    #[test]
    fn unknown_fields_from_a_newer_reclaw_are_ignored() {
        let dir = tempfile::tempdir().expect("dir");
        let file = dir.path().join("installs.json");
        fs::write(&file, r#"{"version": 7, "future": true, "apps": {"k": {"path": "/a", "runner": "proton"}}}"#).expect("write");
        let (book, problem) = InstallBook::load(&file);
        assert!(problem.is_none());
        assert_eq!(book.path_of("k"), Some(Path::new("/a")));
    }

    #[test]
    fn saving_where_nothing_can_be_made_is_an_error_naming_the_place() {
        let dir = tempfile::tempdir().expect("dir");
        fs::write(dir.path().join("blocker"), "x").expect("blocker");
        let (mut book, _) = InstallBook::load(dir.path().join("blocker/installs.json"));
        let error = book.remember("k", Path::new("/a")).expect_err("cannot save");
        assert!(matches!(error, InstallError::Io { .. }), "{error:?}");
    }
}
