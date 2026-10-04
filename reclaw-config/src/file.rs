use std::{
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use crate::{
    error::PrefsError,
    migrate::{Compat, upgrade},
    prefs::Preferences,
};

/// What [`PrefsFile::load`] found.
#[derive(Debug)]
pub struct Loaded {
    pub prefs: Preferences,
    /// Why defaults were used, or what was set aside. Show it once; it is never an error to start.
    pub warning: Option<String>,
    /// The file came from a newer Reclaw: do not save over it.
    pub read_only: bool,
}

/// The settings file at one path.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PrefsFile {
    path: PathBuf,
}

impl PrefsFile {
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Read the file. Never fails: a missing file is the defaults; a file that cannot be parsed is
    /// renamed to `settings.toml.bad` (replacing an older one) and the defaults are used, with a
    /// warning saying so; a file that cannot be read at all is the defaults, left untouched.
    pub fn load(&self) -> Loaded {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => return Loaded { prefs: Preferences::default(), warning: None, read_only: false },
            Err(e) => {
                return Loaded {
                    prefs: Preferences::default(),
                    warning: Some(PrefsError::io(&self.path, e).to_string()),
                    // Unreadable now does not mean empty: do not overwrite what may still be there.
                    read_only: true,
                };
            }
        };
        match toml::from_str::<Preferences>(&text) {
            Ok(mut prefs) => {
                let read_only = match upgrade(&mut prefs) {
                    Compat::Current => false,
                    Compat::Newer { .. } => true,
                };
                let warning = read_only
                    .then(|| PrefsError::TooNew { path: self.path.clone(), found: prefs.version, known: Preferences::VERSION }.to_string());
                Loaded { prefs, warning, read_only }
            }
            Err(e) => {
                let bad = self.bad_path();
                let moved = fs::rename(&self.path, &bad);
                let what = match moved {
                    Ok(()) => {
                        format!("Your settings could not be read ({e}). They were kept as {} and defaults are in use.", bad.display())
                    }
                    Err(rename) => {
                        format!("Your settings could not be read ({e}) and could not be set aside ({rename}). Defaults are in use.")
                    }
                };
                Loaded { prefs: Preferences::default(), warning: Some(what), read_only: false }
            }
        }
    }

    /// Write the file: a temporary file in the same folder, then a rename, so a crash leaves the old
    /// file or the new one. Creates the folder.
    pub fn save(&self, prefs: &Preferences) -> Result<(), PrefsError> {
        let text = toml::to_string_pretty(prefs).map_err(|e| PrefsError::Encode(e.to_string()))?;
        let io = |source| PrefsError::io(&self.path, source);
        let dir = self.path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
        fs::create_dir_all(dir).map_err(io)?;
        let temporary = dir.join(format!(".settings-{}-{}.tmp", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
        let written = fs::File::create(&temporary).and_then(|mut file| {
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
        let renamed = written.and_then(|()| fs::rename(&temporary, &self.path));
        if let Err(source) = renamed {
            // The temporary file is ours alone; the real file was not touched.
            let _ = fs::remove_file(&temporary);
            return Err(io(source));
        }
        Ok(())
    }

    fn bad_path(&self) -> PathBuf {
        let mut name = self.path.file_name().map(|n| n.to_os_string()).unwrap_or_default();
        name.push(".bad");
        self.path.with_file_name(name)
    }
}

static NEXT: AtomicU32 = AtomicU32::new(0);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prefs::PrefValue;

    fn file(dir: &tempfile::TempDir) -> PrefsFile {
        PrefsFile::at(dir.path().join("config").join("settings.toml"))
    }

    #[test]
    fn a_missing_file_is_the_defaults_without_a_warning() {
        let dir = tempfile::tempdir().expect("tempdir");
        let loaded = file(&dir).load();
        assert_eq!(loaded.prefs, Preferences::default());
        assert!(loaded.warning.is_none() && !loaded.read_only);
    }

    #[test]
    fn save_then_load_round_trips_and_creates_the_folder() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = file(&dir);
        let mut prefs = Preferences::default();
        prefs.global.insert("rumble".into(), PrefValue::Bool(false));
        prefs.favorites.insert(9);
        f.save(&prefs).expect("save");
        assert_eq!(f.load().prefs, prefs);
        let leftovers: Vec<_> = fs::read_dir(f.path().parent().expect("parent")).expect("dir").flatten().map(|e| e.file_name()).collect();
        assert_eq!(leftovers.len(), 1, "no temporary file is left behind: {leftovers:?}");
    }

    #[test]
    fn a_damaged_file_is_set_aside_and_defaults_are_used() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = file(&dir);
        fs::create_dir_all(f.path().parent().expect("parent")).expect("mkdir");
        fs::write(f.path(), "this is = = not toml").expect("write");
        let loaded = f.load();
        assert_eq!(loaded.prefs, Preferences::default());
        assert!(loaded.warning.as_deref().is_some_and(|w| w.contains("settings.toml.bad")), "{:?}", loaded.warning);
        assert!(!f.path().exists() && f.path().with_file_name("settings.toml.bad").exists());
        // The next save starts a fresh file; the damaged one is still there to be recovered from.
        f.save(&Preferences::default()).expect("save");
        assert!(f.path().exists());
    }

    #[test]
    fn a_file_from_a_newer_reclaw_is_read_only() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = file(&dir);
        fs::create_dir_all(f.path().parent().expect("parent")).expect("mkdir");
        fs::write(f.path(), "version = 99\nfavorites = [5]\nbrand_new = 1\n").expect("write");
        let loaded = f.load();
        assert!(loaded.read_only);
        assert!(loaded.prefs.favorites.contains(&5), "what this build understands is still used");
        assert!(loaded.warning.is_some());
    }

    #[test]
    fn a_save_overwrites_an_existing_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let f = file(&dir);
        let mut prefs = Preferences::default();
        f.save(&prefs).expect("save");
        prefs.favorites.insert(2);
        f.save(&prefs).expect("save again");
        assert!(f.load().prefs.favorites.contains(&2));
    }
}
