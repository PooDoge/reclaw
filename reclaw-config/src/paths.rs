use std::path::PathBuf;

use directories::ProjectDirs;

const QUALIFIER: &str = "dev";
const ORGANIZATION: &str = "reclaw";
const APPLICATION: &str = "Reclaw";

/// Where Reclaw keeps its files. Config is what the user chose, data is what Reclaw built (the
/// library database), cache is what can be fetched again (art, READMEs).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AppDirs {
    pub config: PathBuf,
    pub data: PathBuf,
    pub cache: PathBuf,
}

impl AppDirs {
    /// The platform's directories (XDG on Linux). `RECLAW_HOME`, when set, puts all three under one
    /// folder: for a portable install, for tests, and for trying Reclaw without touching a real profile.
    pub fn locate(get: impl Fn(&str) -> Option<String>) -> Option<Self> {
        if let Some(home) = get("RECLAW_HOME").filter(|h| !h.is_empty()) {
            let root = PathBuf::from(home);
            return Some(Self { config: root.join("config"), data: root.join("data"), cache: root.join("cache") });
        }
        let dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)?;
        Some(Self { config: dirs.config_dir().to_path_buf(), data: dirs.data_dir().to_path_buf(), cache: dirs.cache_dir().to_path_buf() })
    }

    /// The settings file.
    pub fn prefs_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    /// The user's library of apps (`apps.json`, Quiver's format; see `reclaw-sync`).
    pub fn library_file(&self) -> PathBuf {
        self.data.join("apps.json")
    }

    /// Answers from the catalog and release services, kept so the next start is instant and an offline start still shows something.
    pub fn http_cache(&self) -> PathBuf {
        self.cache.join("http")
    }

    /// Fetched artwork and READMEs live here, one file per address (see the `reclaw-media` crate).
    pub fn media_cache(&self) -> PathBuf {
        self.cache.join("media")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reclaw_home_puts_everything_under_one_root() {
        let dirs = AppDirs::locate(|k| (k == "RECLAW_HOME").then(|| "/tmp/portable".to_string())).expect("dirs");
        assert_eq!(dirs.prefs_file(), PathBuf::from("/tmp/portable/config/settings.toml"));
        assert_eq!(dirs.media_cache(), PathBuf::from("/tmp/portable/cache/media"));
        assert_eq!(dirs.library_file(), PathBuf::from("/tmp/portable/data/apps.json"));
        assert_eq!(dirs.http_cache(), PathBuf::from("/tmp/portable/cache/http"));
    }

    #[test]
    fn an_empty_override_is_ignored() {
        let dirs = AppDirs::locate(|_| Some(String::new()));
        assert!(dirs.is_none_or(|d| !d.config.starts_with("/config")));
    }
}
