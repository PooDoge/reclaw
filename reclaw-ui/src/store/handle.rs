use std::rc::Rc;

use freya::{prelude::*, radio::*};
use reclaw_config::PrefsWriter;

use super::{action::AppAction, channel::AppChannel, feed::StoreInbox, state::AppState};

/// The handle to the shared state. `Copy`, so it moves into any handler or task; it is the only way
/// to change the state. Windows each call [`install`](Self::install) so their components can read.
#[derive(Clone, Copy)]
pub struct Store {
    station: RadioStation<AppState, AppChannel>,
    /// Where changes to saved state are sent. Set once at startup, by [`attach_persistence`](Self::attach_persistence).
    writer: State<Option<Rc<PrefsWriter>>>,
}

impl PartialEq for Store {
    fn eq(&self, other: &Self) -> bool {
        // One store per process (or per test): the writer cell identifies it.
        self.writer == other.writer
    }
}

impl Store {
    /// The process-wide store, for `main`: it lives for the whole run and every window can use it.
    /// Not a hook; call it before `launch`.
    pub fn create_global(initial: AppState) -> Self {
        Self { station: RadioStation::create_global(initial), writer: State::create_global(None) }
    }

    /// A store owned by the calling component, for tests and single-window demos. A hook: call it
    /// once, unconditionally, from the root component.
    pub fn use_scoped(initial: impl FnOnce() -> AppState) -> Self {
        use_hook(|| Self { station: RadioStation::create(initial()), writer: State::create(None) })
    }

    /// Let this window's components read the store. A hook: call it first in each window's root.
    pub fn install(self) {
        use_share_radio(move || self.station);
    }

    /// Save changes to the settings file from now on. Call once, at startup, with the writer for the
    /// file that was loaded. Don't attach one when that file was read-only (written by a newer Reclaw).
    pub fn attach_persistence(self, writer: PrefsWriter) {
        let mut cell = self.writer;
        cell.set(Some(Rc::new(writer)));
    }

    /// Write any saved change now, and wait for it. Call when a window closes: the writer holds a
    /// change back for a moment to merge a burst of them.
    pub fn flush(self) {
        let writer = self.writer.peek().clone();
        if let Some(writer) = writer {
            writer.flush();
        }
    }

    /// Change the state. Readers of the channels the action touched redraw; nobody else does. If a
    /// saved part changed, the settings file is updated shortly after.
    ///
    /// Call from handlers and tasks, not from render, and not while holding a read of the state.
    pub fn dispatch(self, action: AppAction) {
        let mut station = self.station;
        let touched = {
            // Quiet has no readers, so this write notifies nobody; the channels are notified below.
            let mut guard = station.write_channel(AppChannel::Quiet);
            guard.reduce(action)
        };
        for channel in &touched {
            drop(station.write_channel(*channel));
        }
        if touched.iter().any(|c| c.is_persisted()) {
            self.save_now();
        }
    }

    fn save_now(self) {
        let writer = self.writer.peek().clone();
        if let Some(writer) = writer {
            writer.submit(self.with(AppState::to_prefs));
        }
    }

    /// Read the state without subscribing, for handlers.
    pub fn with<R>(self, f: impl FnOnce(&AppState) -> R) -> R {
        let state = self.station.peek();
        f(&state)
    }

    /// A copy of the state, for tests.
    pub fn snapshot(self) -> AppState {
        self.with(AppState::clone)
    }

    /// Dispatch everything that arrives from other threads, until the feed closes. Run it once per
    /// process, as a task on the UI thread.
    pub async fn pump(self, mut inbox: StoreInbox) {
        while let Some(action) = inbox.next().await {
            self.dispatch(action);
        }
    }
}
