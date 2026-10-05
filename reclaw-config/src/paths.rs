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
    /// The log files (`reclaw.log`): state, not configuration, and not something to throw away with the cache.
    pub logs: PathBuf,
}

impl AppDirs {
    /// The platform's directories (XDG on Linux). `RECLAW_HOME`, when set, puts all three under one
    /// folder: for a portable install, for tests, and for trying Reclaw without touching a real profile.
    pub fn locate(get: impl Fn(&str) -> Option<String>) -> Option<Self> {
        if let Some(home) = get("RECLAW_HOME").filter(|h| !h.is_empty()) {
            let root = PathBuf::from(home);
            return Some(Self { config: root.join("config"), data: root.join("data"), cache: root.join("cache"), logs: root.join("logs") });
        }
        let dirs = ProjectDirs::from(QUALIFIER, ORGANIZATION, APPLICATION)?;
        // `state_dir` is XDG's place for logs (`~/.local/state`); the other platforms have no such folder, so the logs sit in
        // the local data folder, where Windows and macOS applications keep them.
        let logs = dirs.state_dir().map_or_else(|| dirs.data_local_dir().join("logs"), |state| state.join("logs"));
        Some(Self {
            config: dirs.config_dir().to_path_buf(),
            data: dirs.data_dir().to_path_buf(),
            cache: dirs.cache_dir().to_path_buf(),
            logs,
        })
    }

    /// The settings file.
    pub fn prefs_file(&self) -> PathBuf {
        self.config.join("settings.toml")
    }

    /// Access tokens (GitHub, GitLab), apart from the settings: a settings file gets pasted into bug reports, this one must not.
    pub fn secrets_file(&self) -> PathBuf {
        self.config.join("secrets.toml")
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
        assert_eq!(dirs.secrets_file(), PathBuf::from("/tmp/portable/config/secrets.toml"));
        assert_eq!(dirs.http_cache(), PathBuf::from("/tmp/portable/cache/http"));
        assert_eq!(dirs.logs, PathBuf::from("/tmp/portable/logs"));
    }

    #[test]
    fn an_empty_override_is_ignored() {
        let dirs = AppDirs::locate(|_| Some(String::new()));
        assert!(dirs.is_none_or(|d| !d.config.starts_with("/config")));
    }
}
