//! Opening the shared store at startup: find the settings file, read it, make the process-wide store
//! and start saving changes to it. The examples and the real app all begin here.
use std::{sync::Arc, time::Duration};

use reclaw_config::{AppDirs, PrefsFile, PrefsWriter};
use reclaw_media::{DiskStore, HttpFetcher, MediaCache, MediaHub, Policy};
use reclaw_net::{Net, NetConfig};

use crate::store::{AppState, Store};

/// How long a change waits for others before the settings file is written.
const SAVE_DELAY: Duration = Duration::from_millis(400);

/// Pictures and documents are fetched this many at a time.
const MEDIA_WORKERS: usize = 4;
/// A request that has not finished in this long has failed.
const MEDIA_TIMEOUT: Duration = Duration::from_secs(20);

/// The program's one HTTP client, configured from the environment (`SSL_CERT_FILE`, `RECLAW_PROXY`, `HTTPS_PROXY`,
/// `GITHUB_TOKEN` ... see `reclaw_net::NetConfig::from_env`), with answers cached under the cache folder. The second part
/// is what was asked for and could not be done, to show once. `None` when the client could not start; the app then runs
/// offline, from whatever is already on disk.
pub fn open_net(dirs: Option<&AppDirs>, get: impl Fn(&str) -> Option<String>) -> (Option<Net>, Vec<String>) {
    let (mut config, mut problems) = NetConfig::from_env(get);
    config.cache_dir = dirs.map(|d| d.http_cache());
    match Net::new(config) {
        Ok(net) => (Some(net), problems),
        Err(e) => {
            problems.push(format!("The network layer could not start, so nothing will be downloaded: {e}"));
            (None, problems)
        }
    }
}

/// Start fetching and caching artwork and READMEs, in the cache folder. `None` when there is no folder to cache in, there
/// is no network layer, or the system would not start the worker threads: the app then shows placeholders, which is how it
/// looks offline anyway.
pub fn open_media(dirs: Option<&AppDirs>, net: Option<&Net>) -> Option<MediaHub> {
    let (dirs, net) = (dirs?, net?);
    let cache =
        MediaCache::new(DiskStore::new(dirs.media_cache()), Arc::new(HttpFetcher::new(net.clone(), MEDIA_TIMEOUT)), Policy::default());
    match MediaHub::start(cache, MEDIA_WORKERS) {
        Ok(hub) => Some(hub),
        Err(e) => {
            eprintln!("reclaw: artwork will not be downloaded: {e}");
            None
        }
    }
}

/// What [`open_store`] made.
pub struct Opened {
    pub store: Store,
    pub dirs: Option<AppDirs>,
    /// Something the user should be told once: the file was damaged, or from a newer Reclaw.
    pub warning: Option<String>,
}

/// Read the settings and create the store. `get` reads environment variables (`RECLAW_HOME` moves
/// every Reclaw folder, for portable installs and for trying things without touching a profile).
/// `seed` fills in what does not come from the settings file: the library, the catalog.
///
/// Not a hook: call it in `main`, before `launch`. If no folder can be found, or the file came from
/// a newer Reclaw, the app still starts, with settings that are not saved.
pub fn open_store(get: impl Fn(&str) -> Option<String>, seed: impl FnOnce(&mut AppState)) -> Opened {
    let dirs = AppDirs::locate(get);
    let file = dirs.as_ref().map(|d| PrefsFile::at(d.prefs_file()));
    let (mut state, read_only, mut warning) = match &file {
        Some(file) => {
            let loaded = file.load();
            (AppState::from_prefs(loaded.prefs), loaded.read_only, loaded.warning)
        }
        None => (AppState::default(), true, Some("No folder for Reclaw's settings was found, so they will not be saved.".to_string())),
    };
    seed(&mut state);
    state.settings_warning = warning.clone();
    let store = Store::create_global(state);
    if let (Some(file), false) = (file, read_only) {
        let writer = PrefsWriter::spawn(file, SAVE_DELAY, |e| eprintln!("reclaw: could not save settings: {e}"));
        if writer.is_running() {
            store.attach_persistence(writer);
        } else {
            warning.get_or_insert_with(|| "Settings could not be saved: the writer thread did not start.".to_string());
        }
    }
    Opened { store, dirs, warning }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        settings::{SettingChange, SettingValue},
        store::AppAction,
    };

    fn home(dir: &tempfile::TempDir) -> impl Fn(&str) -> Option<String> {
        let root = dir.path().display().to_string();
        move |k| (k == "RECLAW_HOME").then(|| root.clone())
    }

    #[test]
    fn a_change_is_saved_and_the_next_start_reads_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        {
            let opened = open_store(home(&dir), |s| *s = AppState { games: crate::sample::sample_games(), ..s.clone() });
            assert!(opened.warning.is_none());
            opened.store.dispatch(AppAction::ToggleFavorite(2));
            opened.store.dispatch(AppAction::Setting(SettingChange { app: None, key: "rumble", value: SettingValue::Bool(false) }));
            opened.store.flush();
        }
        let again = open_store(home(&dir), |_| {});
        let state = again.store.snapshot();
        assert!(state.favorites.contains(&2));
        assert!(!state.settings.toggle(crate::settings::SettingsTarget::Global, "rumble", true));
    }

    #[test]
    fn a_damaged_file_is_set_aside_with_a_warning_and_the_app_still_starts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = dir.path().join("config");
        std::fs::create_dir_all(&config).expect("mkdir");
        std::fs::write(config.join("settings.toml"), "= not toml =").expect("write");
        let opened = open_store(home(&dir), |_| {});
        assert!(opened.warning.as_deref().is_some_and(|w| w.contains("could not be read")), "{:?}", opened.warning);
        assert!(config.join("settings.toml.bad").exists());
    }

    #[test]
    fn a_file_from_a_newer_reclaw_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = dir.path().join("config");
        std::fs::create_dir_all(&config).expect("mkdir");
        std::fs::write(config.join("settings.toml"), "version = 99\nfavorites = [5]\n").expect("write");
        let opened = open_store(home(&dir), |_| {});
        opened.store.dispatch(AppAction::ToggleFavorite(5));
        opened.store.flush();
        let text = std::fs::read_to_string(config.join("settings.toml")).expect("read");
        assert!(text.contains("version = 99"), "left alone: {text}");
    }
}
