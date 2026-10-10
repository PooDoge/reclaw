//! Mods for the installed games: what the sites list for each, what each game's folder records as installed, and installing,
//! updating and removing mods on worker threads reported as activity. The screens get the whole list (`AppAction::SetMods`, and
//! the games it covers in `SetModdable`) each time any of it changes; nothing about mods is kept outside the game folders but the last listings, in memory.
//!
//! * this file: [`Mods`], which games take mods, and building the list the screens show
//! * `entries`: joining a listing with a game's record (pure)
//! * `list`: asking the sites, in the background
//! * `jobs`: installing, updating and removing a mod
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, MutexGuard, atomic::AtomicBool},
};

use reclaw_catalog::mods::ModsConfig;
use reclaw_mods::{Document, ModInstaller, ModSites, Provider};
use reclaw_net::{Cancel, Net};
use reclaw_ui::{
    activity::ActivityId,
    catalog_data::{IdMap, InstallState, key_of},
    model::ModEntry,
    store::AppAction,
};

use crate::host::{Host, install::paths};

mod entries;
mod jobs;
mod list;

/// One mod's job: which game, which site, which mod (lower case).
type JobKey = (String, Provider, String);

struct Running {
    activity: ActivityId,
    cancel: Cancel,
}

/// What the host keeps for mods.
pub struct Mods {
    sites: Option<ModSites>,
    installer: Option<ModInstaller>,
    /// The last listing of each game, by its key: every source's packages, in the sites' order, with their place in each order.
    listings: Mutex<HashMap<String, Vec<entries::Listed>>>,
    jobs: Mutex<HashMap<JobKey, Running>>,
    refreshing: AtomicBool,
}

impl Mods {
    /// `sites` is given in tests to point at a fake server; otherwise the real sites through `net`.
    pub fn new(net: Option<&Net>, sites: Option<ModSites>, downloads: PathBuf) -> Self {
        let sites = sites.or_else(|| net.map(|n| ModSites::new(n.clone())));
        let installer = net.map(|n| ModInstaller::new(n.clone(), downloads));
        Self {
            sites,
            installer,
            listings: Mutex::new(HashMap::new()),
            jobs: Mutex::new(HashMap::new()),
            refreshing: AtomicBool::new(false),
        }
    }

    fn listings(&self) -> MutexGuard<'_, HashMap<String, Vec<entries::Listed>>> {
        self.listings.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn jobs(&self) -> MutexGuard<'_, HashMap<JobKey, Running>> {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn is_busy(&self, game: &str, provider: Provider, id: &str) -> bool {
        self.jobs().contains_key(&(game.to_string(), provider, id.to_ascii_lowercase()))
    }

    /// Stop the mod job that owns this activity. False when none does.
    pub fn cancel(&self, activity: ActivityId) -> bool {
        match self.jobs().values().find(|job| job.activity == activity) {
            Some(job) => {
                job.cancel.cancel();
                true
            }
            None => false,
        }
    }
}

/// A game that takes mods: installed, with a folder on disk and a usable mod configuration.
#[derive(Clone, Debug)]
pub(super) struct Game {
    id: u32,
    key: String,
    title: String,
    folder: PathBuf,
    config: ModsConfig,
    /// The catalog's `filesToAdd`: a recomp loads mods only once `portable.txt` is there.
    markers: Vec<String>,
}

impl Host {
    /// The library's games that take mods now.
    fn moddable(&self) -> Vec<Game> {
        let candidates: Vec<Game> = {
            let state = self.state();
            let ids = IdMap::for_keys(state.catalog.iter().map(|a| key_of(&a.entry)).chain(state.library.iter().map(key_of)));
            state
                .library
                .iter()
                .filter_map(|entry| {
                    let key = key_of(entry);
                    let installed = matches!(state.installs.get(&key), Some(InstallState::Installed { .. }));
                    if !installed && !entry.is_manual() {
                        return None;
                    }
                    let catalog = state.catalog.iter().find(|a| key_of(&a.entry) == key).map(|a| &a.entry);
                    let config = entries::config_for(&entry.mods, catalog.map(|c| &c.mods))?.clone();
                    let markers = if entry.files_to_add.is_empty() {
                        catalog.map(|c| c.files_to_add.clone()).unwrap_or_default()
                    } else {
                        entry.files_to_add.clone()
                    };
                    let folder = paths::folder_of(entry, &state.default_location, self.inner.installs.home.as_deref()).ok()?;
                    Some(Game { id: ids.get(&key)?, key, title: entry.name.clone(), folder, config, markers })
                })
                .collect()
        };
        candidates.into_iter().filter(|game| game.folder.is_dir()).collect()
    }

    /// The list the screens show, from the last listings and what each game's folder records.
    pub(crate) fn mod_entries(&self) -> Vec<ModEntry> {
        self.mod_entries_of(&self.moddable())
    }

    fn mod_entries_of(&self, games: &[Game]) -> Vec<ModEntry> {
        let listings = self.inner.mods.listings().clone();
        let mut all = Vec::new();
        for game in games {
            let records = match Document::load(&game.folder) {
                Ok(document) => document.mods,
                Err(error) => {
                    // Shown as nothing installed; an install refuses to touch the folder until the file is fixed, and says so.
                    tracing::warn!(game = %game.title, folder = %game.folder.display(), %error, "the record of installed mods could not be read");
                    Vec::new()
                }
            };
            let listing = listings.get(&game.key).map(Vec::as_slice).unwrap_or_default();
            let busy = |provider: Provider, id: &str| self.inner.mods.is_busy(&game.key, provider, id);
            all.extend(entries::entries(game.id, listing, &records, &busy));
        }
        all
    }

    /// Send the screens the list as it is now.
    /// The games are sent too, so the Mods tab can offer a game whose sites have listed nothing yet.
    pub(crate) fn publish_mods(&self) {
        let games = self.moddable();
        self.send(AppAction::SetModdable(games.iter().map(|g| g.id).collect()));
        self.send(AppAction::SetMods(self.mod_entries_of(&games)));
    }
}

#[cfg(test)]
mod tests;
