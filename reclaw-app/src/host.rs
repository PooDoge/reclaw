//! The part of the program that does what the screens ask: loading the catalog (in the background, so the window never waits for the
//! network), keeping the library file, opening links, and saying plainly when something is not built yet. It owns no windows; it
//! answers [`Effect`]s and sends [`AppAction`]s back through a [`Sink`], so all of it is tested without a window.
use std::{
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};

use reclaw_catalog::AppEntry;
use reclaw_net::Net;
use reclaw_sync::{CatalogApp, CatalogSnapshot, CatalogSync, LibraryError, LibraryStore};
use reclaw_ui::{
    catalog_data::{self, IdMap, Loaded, key_of},
    effect::Effect,
    notices::Notice,
    store::{AppAction, CatalogPhase, CatalogStatus, StoreFeed},
};

use crate::browse;

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

/// Everything the screens start with.
pub struct Initial {
    pub loaded: Loaded,
    pub status: CatalogStatus,
    /// Things to tell the user once the window is up (the library could not be read, ...).
    pub notices: Vec<Notice>,
}

struct State {
    library: Vec<AppEntry>,
    catalog: Vec<CatalogApp>,
    /// False when the library file could not be read: it is left exactly as it is, and nothing is saved over it.
    library_writable: bool,
    status: CatalogStatus,
}

struct Inner {
    sync: Option<CatalogSync>,
    store: LibraryStore,
    sink: Arc<dyn Sink>,
    state: Mutex<State>,
    refreshing: AtomicBool,
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
    /// Read the library, and whatever catalog was saved by an earlier run (no network), so the first frame already has data.
    /// `net` is `None` when the network layer could not start: the program then runs from what is on disk.
    pub fn open(net: Option<Net>, library_file: std::path::PathBuf, index_url: Option<String>, sink: Arc<dyn Sink>) -> (Self, Initial) {
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
        let sync = net.map(|net| {
            let sync = CatalogSync::new(net);
            match index_url {
                Some(url) => sync.with_index_url(url),
                None => sync,
            }
        });
        let saved = sync.as_ref().and_then(CatalogSync::saved);
        let (catalog, status) = match &saved {
            Some(snapshot) => (snapshot.apps(), status_from(snapshot)),
            None => (Vec::new(), CatalogStatus::default()),
        };
        let loaded = catalog_data::load(&catalog, &library);
        let host = Self {
            inner: Arc::new(Inner {
                sync,
                store,
                sink,
                state: Mutex::new(State { library, catalog, library_writable, status: status.clone() }),
                refreshing: AtomicBool::new(false),
            }),
        };
        (host, Initial { loaded, status, notices })
    }

    fn state(&self) -> MutexGuard<'_, State> {
        // The data behind the lock is plain and consistent between statements; a panic elsewhere does not make it wrong.
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
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
                let loaded = {
                    let mut state = self.state();
                    state.catalog = snapshot.apps();
                    state.status = status.clone();
                    catalog_data::load(&state.catalog, &state.library)
                };
                self.send(AppAction::SetProjects(loaded.projects));
                self.send(AppAction::SetGames(loaded.games));
                self.send(AppAction::Catalog(status.clone()));
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
        match effect {
            Effect::RefreshCatalog => self.refresh(),
            Effect::AddToLibrary(id) => {
                self.add_to_library(*id);
            }
            Effect::RemoveFromLibrary(id) => self.remove_from_library(*id),
            Effect::OpenUrl(url) => {
                if let Err(error) = browse::open_url(url) {
                    self.tell(Notice::problem("A link could not be opened", &error.to_string(), vec![]));
                }
            }
            // An install keeps the app in the library; the install itself is the next milestone.
            Effect::StartInstall { app, .. } => {
                let title = self.add_to_library(*app);
                self.not_yet("Installing", title.as_deref());
            }
            Effect::Update(app)
            | Effect::Launch(app)
            | Effect::Resume(app)
            | Effect::Uninstall(app)
            | Effect::Verify(app)
            | Effect::CheckUpdate(app)
            | Effect::OpenFolder(app)
            | Effect::ChooseFile(app) => {
                let title = self.title_of(*app);
                self.not_yet(what_is_missing(effect), title.as_deref());
            }
            Effect::InstallMod { .. } | Effect::RemoveMod { .. } => self.not_yet("Installing mods", None),
            // The rest is the shell's (pages, the window, settings, text boxes) or the gamepad's.
            _ => {}
        }
    }

    fn not_yet(&self, what: &str, title: Option<&str>) {
        let body = match title {
            Some(title) => format!("{what} is not available yet; {title} is in your library"),
            None => format!("{what} is not available yet"),
        };
        self.tell(Notice::note(
            &format!("{what} is not built yet"),
            &body,
            vec!["It is the next milestone: see docs/quiver-parity.md.".to_string()],
        ));
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
                let games = catalog_data::load(&state.catalog, &state.library).games;
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
                let games = catalog_data::load(&state.catalog, &state.library).games;
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

fn what_is_missing(effect: &Effect) -> &'static str {
    match effect {
        Effect::Update(_) => "Updating",
        Effect::Launch(_) | Effect::Resume(_) => "Launching",
        Effect::Uninstall(_) => "Uninstalling",
        Effect::Verify(_) => "Verifying files",
        Effect::CheckUpdate(_) => "Checking for updates",
        Effect::OpenFolder(_) => "Opening the install folder",
        _ => "Choosing a game file",
    }
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
