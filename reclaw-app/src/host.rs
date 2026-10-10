//! The part of the program that does what the screens ask: loading the catalog (in the background, so the window never waits for the
//! network), keeping the library file and the access tokens, opening links and folders, writing the diagnostics report, and the
//! games' mods. It owns no windows; it answers [`Effect`]s and sends [`AppAction`]s back through a [`Sink`], so all of it is
//! tested without a window.
//!
//! * this file: [`Host`], [`HostConfig`], the catalog refresh and the library
//! * `credentials`: saving, checking and removing access tokens, and telling the screens what each service allows
//! * `report`: the diagnostics report
//! * `update`: updating a development copy from the git checkout it was built from
//! * `install`: installing, updating, uninstalling and checking apps (one thread per job, reported as activity)
//! * `launch`: starting and stopping apps, through a Windows runner when needed, and following how they end
//! * `mods`: listing, installing, updating and removing the mods of installed games
//! * `community`: what quiverlauncher.com says about each game (ratings, reviews, verified releases), linked after each refresh and read
//!   when a page opens; library apps following the site's name, icon and tags
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use reclaw_catalog::AppEntry;
use reclaw_install::{Api, Installer, Platform, ReleaseSource};
use reclaw_log::{LogLevel, Logging};
use reclaw_net::Net;
use reclaw_sync::{CatalogApp, CatalogSnapshot, CatalogSync, LibraryError, LibraryStore};
use reclaw_ui::{
    catalog_data::{self, IdMap, InstallState, InstallStates, Loaded, key_of},
    credentials::CredentialsStatus,
    effect::Effect,
    launch_request::LaunchRequest,
    notices::Notice,
    settings::{FALLBACK_LOCATION, KEY_LOG_LEVEL, TextField},
    store::{AppAction, CatalogPhase, CatalogStatus, StoreFeed},
};

use crate::browse;

mod community;
mod credentials;
mod install;
mod launch;
mod mods;
mod report;
mod update;

pub use credentials::{Checker, EnvToken};
pub use update::UpdateSource;

/// Where actions for the screens go. The real one is the store's feed; tests collect them.
pub trait Sink: Send + Sync {
    fn send(&self, action: AppAction);
}

impl Sink for StoreFeed {
    fn send(&self, action: AppAction) {
        // `false` means the window has closed and nobody is listening: nothing to do.
        StoreFeed::send(self, action);
    }
}

/// What the host is given to work with. Everything but the library file is optional: with no network layer the program runs from
/// what is on disk, with no folder for tokens a token lasts for the session, with no log the report simply has no log in it.
pub struct HostConfig {
    pub net: Option<Net>,
    pub library_file: PathBuf,
    /// Another catalog index than the community one.
    pub index_url: Option<String>,
    /// The tokens file. `None`: no folder was found, so a pasted token is used for this run only.
    pub secrets_file: Option<PathBuf>,
    pub logs_dir: Option<PathBuf>,
    pub logging: Option<Arc<Logging>>,
    /// Tokens the environment supplies (`GITHUB_TOKEN`...): used when nothing is saved.
    pub env_tokens: Vec<EnvToken>,
    /// How a service is asked about its token. `None`: over the network. A test supplies its own.
    pub checker: Option<Checker>,
    /// The git checkout this copy was built from, if it is still there: what Update from source works on.
    pub update: Option<UpdateSource>,
    /// Where downloads wait until they are installed (a cut-off one continues from here). `None`: the system's temporary folder.
    pub downloads_dir: Option<PathBuf>,
    /// The Library setting "Default install location", as saved. `None` or empty: `~/Reclaw/Apps`.
    pub default_location: Option<String>,
    /// The person's home folder, for `~`.
    pub home: Option<PathBuf>,
    /// Folders an uninstall must never delete or contain: Reclaw's own.
    pub protect: Vec<PathBuf>,
    /// How long a graceful Stop may take before the app is killed. `None`: eight seconds.
    pub grace: Option<std::time::Duration>,
    /// Where to look for Wine and Proton. `None`: the search path and the home folder.
    pub probe: Option<reclaw_runtime::Probe>,
    /// Another release API than GitHub's and GitLab's (a test's own server).
    pub api: Option<Api>,
    /// The platform to install builds for. `None`: the one this program was built for.
    pub platform: Option<Platform>,
    /// Other mod sites than Thunderstore and GameBanana (a test's own server).
    pub mod_sites: Option<reclaw_mods::ModSites>,
    /// Another quiverlauncher.com API than the real one (`QUIVER_API`, or a test's own server).
    pub site_api: Option<String>,
}

impl HostConfig {
    pub fn new(net: Option<Net>, library_file: PathBuf) -> Self {
        Self {
            net,
            library_file,
            index_url: None,
            secrets_file: None,
            logs_dir: None,
            logging: None,
            env_tokens: Vec::new(),
            checker: None,
            update: None,
            downloads_dir: None,
            default_location: None,
            home: None,
            protect: Vec::new(),
            grace: None,
            probe: None,
            api: None,
            platform: None,
            mod_sites: None,
            site_api: None,
        }
    }
}

/// Everything the screens start with.
pub struct Initial {
    pub loaded: Loaded,
    pub status: CatalogStatus,
    /// Which services have a token and where it came from (not yet asked of the services; see `Host::check_tokens`).
    pub credentials: CredentialsStatus,
    /// Things to tell the user once the window is up (the library could not be read, ...).
    pub notices: Vec<Notice>,
    /// The installed mods, from the game folders (the sites are asked once the catalog is refreshed).
    pub mods: Vec<reclaw_ui::model::ModEntry>,
}

struct State {
    library: Vec<AppEntry>,
    catalog: Vec<CatalogApp>,
    /// False when the library file could not be read: it is left exactly as it is, and nothing is saved over it.
    library_writable: bool,
    status: CatalogStatus,
    /// What the installer knows about each app, by key; an app with no entry is not installed.
    installs: InstallStates,
    default_location: String,
}

struct Inner {
    net: Option<Net>,
    sync: Option<CatalogSync>,
    store: LibraryStore,
    sink: Arc<dyn Sink>,
    state: Mutex<State>,
    refreshing: AtomicBool,
    tokens: credentials::TokenBook,
    checker: Option<Checker>,
    update: Option<UpdateSource>,
    updating: AtomicBool,
    logs_dir: Option<PathBuf>,
    logging: Option<Arc<Logging>>,
    installs: install::Installs,
    mods: mods::Mods,
    community: community::Community,
    runs: launch::Runs,
    protected: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct Host {
    inner: Arc<Inner>,
}

impl PartialEq for Host {
    /// Two handles are equal when they are the same host.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

fn status_from(snapshot: &CatalogSnapshot) -> CatalogStatus {
    CatalogStatus {
        phase: CatalogPhase::Ready,
        updated_at: Some(snapshot.fetched_at),
        stale: snapshot.has_stale_parts(),
        problems: snapshot.problems.iter().map(|p| format!("{} {}", p.what, p.detail)).collect(),
    }
}

impl Host {
    /// Read the library, the saved tokens and whatever catalog was saved by an earlier run (no network), so the first frame already
    /// has data. `config.net` is `None` when the network layer could not start: the program then runs from what is on disk.
    pub fn open(config: HostConfig, sink: Arc<dyn Sink>) -> (Self, Initial) {
        let HostConfig {
            net,
            library_file,
            index_url,
            secrets_file,
            logs_dir,
            logging,
            env_tokens,
            checker,
            update,
            downloads_dir,
            default_location,
            home,
            protect,
            grace,
            probe,
            api,
            platform,
            mod_sites,
            site_api,
        } = config;
        let store = LibraryStore::new(library_file);
        let mut notices = Vec::new();
        let (library, library_writable) = match store.load() {
            Ok(library) => (library, true),
            Err(error) => {
                notices.push(Notice::problem(
                    "Your library could not be read",
                    "Reclaw left it exactly as it is and will not change it",
                    vec![error.to_string(), "Fix or move the file, or ask Reclaw to set it aside and start a new library.".to_string()],
                ));
                (Vec::new(), false)
            }
        };
        let (tokens, token_notices) = credentials::TokenBook::open(secrets_file, env_tokens, net.as_ref());
        notices.extend(token_notices);
        let credentials = tokens.status();
        let community = community::Community::new(net.as_ref(), site_api);
        let sync = net.clone().map(|net| {
            let sync = CatalogSync::new(net);
            let sync = match community.client() {
                Some(site) => sync.with_site(site),
                None => sync,
            };
            match index_url {
                Some(url) => sync.with_index_url(url),
                None => sync,
            }
        });
        let saved = sync.as_ref().and_then(CatalogSync::saved);
        let (catalog, status) = match &saved {
            Some(snapshot) => (snapshot.apps_for(&library), status_from(snapshot)),
            None => (Vec::new(), CatalogStatus::default()),
        };
        let default_location =
            default_location.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).unwrap_or_else(|| FALLBACK_LOCATION.to_string());
        let platform = platform.unwrap_or_else(Platform::detect);
        let downloads_dir = downloads_dir.unwrap_or_else(|| std::env::temp_dir().join("reclaw-downloads"));
        let mods = mods::Mods::new(net.as_ref(), mod_sites, downloads_dir.join("mods"));
        let installer = net.clone().map(|net| {
            let source = ReleaseSource::new(net.clone()).with_api(api.unwrap_or_default());
            Installer::new(net, source, downloads_dir)
        });
        let installs = scan_installs(&library, &default_location, home.as_deref());
        let (supervisor, session_events) = reclaw_runtime::Supervisor::new(grace.unwrap_or(launch::STOP_GRACE));
        let loaded = catalog_data::load(&catalog, &library, &installs);
        let host = Self {
            inner: Arc::new(Inner {
                net,
                sync,
                store,
                sink,
                state: Mutex::new(State { library, catalog, library_writable, status: status.clone(), installs, default_location }),
                refreshing: AtomicBool::new(false),
                tokens,
                checker,
                update,
                updating: AtomicBool::new(false),
                logs_dir,
                logging,
                installs: install::Installs::new(installer, platform, home.clone()),
                mods,
                community,
                runs: {
                    let runs = launch::Runs::new(supervisor, home.as_deref());
                    match probe {
                        Some(probe) => runs.with_probe(probe),
                        None => runs,
                    }
                },
                protected: protect,
            }),
        };
        host.watch_sessions(session_events);
        host.watch_for_refused_tokens();
        // What the site said last time, until the first refresh says it again: ratings and verified releases work offline.
        if let Some(site) = saved.as_ref().and_then(|s| s.site.as_ref()) {
            host.publish_community(&site.links);
        }
        // Read under one lock: a second `state()` in the same statement would wait for the first for ever.
        let (apps_in_library, apps_in_catalog) = {
            let state = host.state();
            (state.library.len(), state.catalog.len())
        };
        tracing::info!(
            library = apps_in_library,
            catalog = apps_in_catalog,
            github = ?credentials.github.source,
            gitlab = ?credentials.gitlab.source,
            "host opened"
        );
        let mods = host.mod_entries();
        (host, Initial { loaded, status, credentials, notices, mods })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // The data behind the lock is plain and consistent between statements; a panic elsewhere does not make it wrong.
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn default_location(&self) -> String {
        self.state().default_location.clone()
    }

    /// The Library setting changed: new installs start there, and apps with no recorded folder are looked for there (so pointing it at
    /// a folder of existing installs, Quiver's for one, adopts them). Apps installed by Reclaw stay where they were put.
    fn set_default_location(&self, text: &str) {
        let text = text.trim();
        self.state().default_location = if text.is_empty() { FALLBACK_LOCATION.to_string() } else { text.to_string() };
        self.rescan_installs();
    }

    /// Read the folders again and show the library as they are. A job that is running keeps showing as running.
    fn rescan_installs(&self) {
        let games = {
            let mut guard = self.state();
            let fresh = scan_installs(&guard.library, &guard.default_location, self.inner.installs.home.as_deref());
            let mut next = guard.installs.clone();
            for entry in guard.library.iter().filter(|entry| !entry.is_manual()) {
                let key = key_of(entry);
                match (guard.installs.get(&key), fresh.get(&key)) {
                    (Some(InstallState::Installing), _) | (Some(InstallState::Failed), None) => {}
                    (_, Some(found)) => {
                        next.insert(key, found.clone());
                    }
                    (_, None) => {
                        next.remove(&key);
                    }
                }
            }
            if next == guard.installs {
                return;
            }
            guard.installs = next;
            catalog_data::load(&guard.catalog, &guard.library, &guard.installs).games
        };
        self.send(AppAction::SetGames(games));
    }

    /// Record what the installer now knows about an app (`None`: not installed) and show the library that way.
    pub(crate) fn set_install_state(&self, key: &str, state: Option<InstallState>) {
        let games = {
            let mut guard = self.state();
            match state {
                Some(next) => guard.installs.insert(key.to_string(), next),
                None => guard.installs.remove(key),
            };
            catalog_data::load(&guard.catalog, &guard.library, &guard.installs).games
        };
        self.send(AppAction::SetGames(games));
    }

    fn send(&self, action: AppAction) {
        self.inner.sink.send(action);
    }

    fn tell(&self, notice: Notice) {
        self.send(AppAction::Notify(notice));
    }

    /// Load the catalog again, in the background. A second request while one runs does nothing.
    pub fn refresh(&self) {
        let Some(sync) = self.inner.sync.clone() else {
            self.tell(Notice::problem("No network", "The network layer could not start, so the catalog cannot be refreshed", vec![]));
            return;
        };
        if self.inner.refreshing.swap(true, Ordering::SeqCst) {
            return;
        }
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-catalog".into()).spawn({
            let host = host.clone();
            move || {
                host.run_refresh(&sync);
                host.inner.refreshing.store(false, Ordering::SeqCst);
            }
        });
        if let Err(error) = started {
            self.inner.refreshing.store(false, Ordering::SeqCst);
            self.tell(Notice::problem("The catalog could not be refreshed", "A background thread did not start", vec![error.to_string()]));
        }
    }

    /// The body of a refresh: runs on its own thread, and also directly in tests.
    pub fn run_refresh(&self, sync: &CatalogSync) {
        let previous = self.state().status.clone();
        self.send(AppAction::Catalog(CatalogStatus::loading(&previous)));
        match sync.refresh() {
            Ok(snapshot) => {
                let status = status_from(&snapshot);
                if let Some(site) = &snapshot.site {
                    // Library apps from the site take its current name, project, icon and tags, unless the person changed them.
                    self.follow_site(&site.links);
                }
                let loaded = {
                    let mut state = self.state();
                    state.catalog = snapshot.apps_for(&state.library);
                    state.status = status.clone();
                    catalog_data::load(&state.catalog, &state.library, &state.installs)
                };
                self.send(AppAction::SetProjects(loaded.projects));
                self.send(AppAction::SetGames(loaded.games));
                self.send(AppAction::Catalog(status.clone()));
                // The catalog says which games take mods and where their mods are listed.
                self.refresh_mods(false);
                // The games are known: link them to what quiverlauncher.com says about them (read with the catalog).
                if let Some(site) = &snapshot.site {
                    self.publish_community(&site.links);
                }
                if status.stale || !status.problems.is_empty() {
                    let title =
                        if status.stale { "Showing a saved copy of the catalog" } else { "Part of the catalog could not be loaded" };
                    self.tell(Notice::note(title, "Open this for what happened", status.problems.clone()));
                }
            }
            Err(error) => {
                let have_data = !self.state().catalog.is_empty();
                let status = CatalogStatus {
                    phase: if have_data { CatalogPhase::Ready } else { CatalogPhase::Failed },
                    stale: have_data,
                    problems: vec![error.to_string()],
                    ..previous
                };
                self.state().status = status.clone();
                self.send(AppAction::Catalog(status));
                let mut details = vec![error.to_string()];
                details.extend(error.hint().map(str::to_string));
                let body = if have_data { "Showing the saved copy" } else { "There is no saved copy to show" };
                self.tell(Notice::problem("The catalog could not be refreshed", body, details));
            }
        }
    }

    /// Answer one effect. The ones the shell handles itself (navigation, the window, settings) are not here.
    pub fn handle(&self, effect: &Effect) {
        // The effect's name, not its contents: some carry text (a launch option) and the log needs only what was asked.
        let text = format!("{effect:?}");
        tracing::debug!(effect = text.split(['(', ' ', '{']).next().unwrap_or_default(), "asked of the host");
        match effect {
            Effect::RefreshCatalog => self.refresh(),
            Effect::SaveToken { provider, token } => self.save_token(*provider, token),
            Effect::RemoveToken(provider) => self.remove_token(*provider),
            Effect::CheckToken(provider) => self.spawn_check(*provider, credentials::Reason::Asked),
            Effect::OpenLogFolder => match &self.inner.logs_dir {
                Some(dir) => {
                    if let Err(error) = browse::open_folder(dir) {
                        tracing::warn!(dir = %dir.display(), %error, "the log folder could not be opened");
                        self.tell(Notice::problem(
                            "The log folder could not be opened",
                            &error.to_string(),
                            vec![format!("It is {}", dir.display())],
                        ));
                    }
                }
                None => self.tell(Notice::problem("There is no log folder", "No folder for Reclaw's files was found", vec![])),
            },
            Effect::SaveDiagnostics => self.save_report(),
            Effect::UpdateSources => self.update_sources(),
            Effect::Setting(change) if change.app.is_none() && change.key == KEY_LOG_LEVEL => {
                if let (Some(logging), reclaw_ui::settings::SettingValue::Choice(i)) = (&self.inner.logging, change.value) {
                    let level = LogLevel::from_index(i);
                    tracing::info!(?level, "the log level was changed in Settings");
                    logging.set_level(level);
                }
            }
            Effect::AddToLibrary(id) => {
                self.add_to_library(*id);
            }
            Effect::RemoveFromLibrary(id) => self.remove_from_library(*id),
            Effect::OpenUrl(url) => {
                if let Err(error) = browse::open_url(url) {
                    tracing::warn!(%error, "a link could not be opened");
                    self.tell(Notice::problem("A link could not be opened", &error.to_string(), vec![]));
                }
            }
            Effect::StartInstall { app, location, prerelease } => self.start_install(*app, Some(location), *prerelease),
            Effect::Update(app) => self.start_install(*app, None, false),
            Effect::Uninstall(app) => self.uninstall(*app),
            Effect::OpenFolder(app) => self.open_app_folder(*app),
            Effect::CheckUpdate(app) => self.check_update(*app),
            Effect::Verify(app) => self.verify(*app),
            Effect::CancelActivity(id) => self.cancel_activity(*id),
            Effect::TextCommitted { app: None, field: TextField::DefaultLocation, value } => self.set_default_location(value),
            // The entry point attaches the person's launch settings (`launch_request`) and calls `launch` itself; a plain press has none.
            Effect::Launch(app) => self.launch(*app, &LaunchRequest::default()),
            Effect::Stop(app) => self.stop(*app),
            // Bringing a running app forward is the window system's business (and the pad's, which the shell has already given it).
            Effect::Resume(app) => tracing::debug!(app, "Resume: the app already has the screen"),
            Effect::InstallMod { game, provider, id } => self.install_mod(*game, *provider, id),
            Effect::RemoveMod { game, provider, id } => self.remove_mod(*game, *provider, id),
            Effect::RefreshMods => self.refresh_mods(true),
            Effect::LoadCommunity(app) => self.load_community_page(*app),
            // The rest is the shell's (pages, the window, settings, text boxes) or the gamepad's.
            _ => {}
        }
    }

    fn title_of(&self, id: u32) -> Option<String> {
        let state = self.state();
        let (_, entry) = find(&state, id)?;
        Some(entry.name.clone())
    }

    /// Put a catalog app in the library, save it, and tell the screens. The app's title, or `None` if the id is unknown or the
    /// library cannot be written (and the user has been told).
    fn add_to_library(&self, id: u32) -> Option<String> {
        let mut state = self.state();
        let (in_library, entry) = find(&state, id)?;
        let title = entry.name.clone();
        if in_library {
            return Some(title);
        }
        if !state.library_writable {
            drop(state);
            self.tell(read_only());
            return None;
        }
        let mut next = state.library.clone();
        next.push(entry.for_library());
        match self.inner.store.save(&next) {
            Ok(()) => {
                state.library = next;
                let games = catalog_data::load(&state.catalog, &state.library, &state.installs).games;
                drop(state);
                self.send(AppAction::SetGames(games));
                Some(title)
            }
            Err(error) => {
                drop(state);
                self.tell(save_failed(&error));
                None
            }
        }
    }

    fn remove_from_library(&self, id: u32) {
        let mut state = self.state();
        let Some((true, entry)) = find(&state, id) else { return };
        if !state.library_writable {
            drop(state);
            self.tell(read_only());
            return;
        }
        let next: Vec<AppEntry> = state.library.iter().filter(|a| !a.same_instance(&entry)).cloned().collect();
        match self.inner.store.save(&next) {
            Ok(()) => {
                state.library = next;
                let games = catalog_data::load(&state.catalog, &state.library, &state.installs).games;
                drop(state);
                self.send(AppAction::SetGames(games));
            }
            Err(error) => {
                drop(state);
                self.tell(save_failed(&error));
            }
        }
    }
}

/// The app with this id, from the library if it is there and the catalog otherwise, and whether it was in the library.
fn find(state: &State, id: u32) -> Option<(bool, AppEntry)> {
    let ids = IdMap::for_keys(state.catalog.iter().map(|a| key_of(&a.entry)).chain(state.library.iter().map(key_of)));
    state
        .library
        .iter()
        .find(|a| ids.get(&key_of(a)) == Some(id))
        .map(|a| (true, a.clone()))
        .or_else(|| state.catalog.iter().find(|a| ids.get(&key_of(&a.entry)) == Some(id)).map(|a| (false, a.entry.clone())))
}

/// What is installed, read from the folders (a version file and no unfinished-install marker): cheap enough to do for the whole
/// library at start. Whether a program is really there is checked when it matters (Play, Verify).
fn scan_installs(library: &[AppEntry], default_location: &str, home: Option<&std::path::Path>) -> InstallStates {
    library
        .iter()
        .filter(|entry| !entry.is_manual())
        .filter_map(|entry| {
            let folder = install::paths::folder_of(entry, default_location, home).ok()?;
            let version = reclaw_install::layout::installed_version(&folder)?;
            Some((key_of(entry), InstallState::Installed { version, latest: None }))
        })
        .collect()
}

fn read_only() -> Notice {
    Notice::problem(
        "Your library is read-only",
        "Its file could not be read, so Reclaw will not change it",
        vec!["Fix or move apps.json, then restart.".to_string()],
    )
}

fn save_failed(error: &LibraryError) -> Notice {
    Notice::problem("Your library could not be saved", &error.to_string(), vec![error.to_string()])
}

#[cfg(test)]
mod tests;
