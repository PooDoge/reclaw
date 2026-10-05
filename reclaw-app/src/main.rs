//! `reclaw`: the launcher. It opens the settings, starts the network layer, shows what the last run saved of the catalog and
//! the library at once, refreshes the catalog in the background, and then runs the window. Everything it does for the screens is
//! in `host.rs`; this file only wires things together.
//!
//! Environment (see `docs/BUILDING.md`): `RECLAW_HOME` (where its folders are), `RECLAW_MODE=deck|desktop`,
//! `RECLAW_CATALOG_INDEX=<url>` (another catalog index), `RECLAW_WINDOW_FRAME=native`, `SSL_CERT_FILE`, `HTTPS_PROXY`,
//! `RECLAW_PROXY`, `GITHUB_TOKEN`. `--open /game/4` starts on a page. F10 switches interface, F9 simulates the on-screen keyboard.
use std::sync::{Arc, Mutex};

use freya::prelude::*;
use reclaw_app::{Host, Sink};
use reclaw_config::AppDirs;
use reclaw_input::{ActionMap, Button, GuideOwner, detect_environment};
use reclaw_ui::{
    bootstrap::{open_media, open_net, open_store},
    deck::ActionFeed,
    effect::Effect,
    nav::Route,
    prelude::*,
    shell::{DevOverrides, Services, Shell},
    store::{AppAction, Store, StoreInbox, feed},
    window::{Frame, WindowHost, detect_server, launch::launch_config},
};

/// Something handed once to the first render, shared by the clones the toolkit makes of the root component.
struct Handoff<T>(Arc<Mutex<Option<T>>>);

impl<T> Clone for Handoff<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Handoff<T> {
    fn new(value: T) -> Self {
        Self(Arc::new(Mutex::new(Some(value))))
    }

    fn take(&self) -> Option<T> {
        self.0.lock().unwrap_or_else(|e| e.into_inner()).take()
    }
}

impl<T> PartialEq for Handoff<T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[derive(Clone, PartialEq)]
struct Launcher {
    frame: Frame,
    media: Option<reclaw_media::MediaHub>,
    store: Store,
    pad: ActionFeed,
    map: ActionMap,
    mode: reclaw_input::Environment,
    host: Host,
    inbox: Handoff<StoreInbox>,
    start: Route,
}

/// The gamepad reader's handle, when there is one.
#[derive(Clone)]
struct Pad {
    #[cfg(feature = "gamepad")]
    input: Option<reclaw_input::backend::InputHandle>,
}

/// Read the gamepad on its own thread and forward what it says to the shell and the store. Without the `gamepad` feature there is
/// no reader and the keyboard and mouse drive everything.
#[cfg(feature = "gamepad")]
fn start_pad(map: ActionMap, pad: &ActionFeed, store: Store) -> Pad {
    use futures_util::StreamExt;
    use reclaw_input::backend::{self, InputMessage};
    use reclaw_ui::notices::hold_rules;

    let (input, messages) = match backend::spawn_with(map, hold_rules()) {
        Ok((input, messages)) => (Some(input), Some(messages)),
        Err(e) => {
            eprintln!("reclaw: the gamepad reader did not start ({e}); the keyboard still works");
            (None, None)
        }
    };
    let tx = pad.sender();
    spawn(async move {
        let Some(mut messages) = messages else { return };
        while let Some(message) = messages.next().await {
            match message {
                InputMessage::Action(a) => {
                    let _ = tx.unbounded_send(a);
                }
                InputMessage::Connected(info) => store.dispatch(AppAction::SetController(Some(info))),
                InputMessage::Disconnected { .. } => store.dispatch(AppAction::SetController(None)),
                InputMessage::Unavailable(why) => eprintln!("reclaw: gamepad unavailable: {why}"),
            }
        }
    });
    Pad { input }
}

#[cfg(not(feature = "gamepad"))]
fn start_pad(_map: ActionMap, _pad: &ActionFeed, _store: Store) -> Pad {
    Pad {}
}

impl App for Launcher {
    fn render(&self) -> impl IntoElement {
        let dev = DevOverrides::from_env(|k| std::env::var(k).ok());
        let store = self.store;
        use_init_reclaw(dev.theme.unwrap_or_else(|| store.with(|s| ThemeKind::from_settings(&s.settings))));

        // Started once: the store's inbox (other threads reach the store through it), the gamepad, the first catalog refresh.
        let pad = use_hook({
            let (map, pad, host, inbox) = (self.map.clone(), self.pad.clone(), self.host.clone(), self.inbox.clone());
            move || {
                if let Some(inbox) = inbox.take() {
                    spawn(async move { store.pump(inbox).await });
                }
                host.refresh();
                start_pad(map, &pad, store)
            }
        });

        let host = self.host.clone();
        let on_effect = EventHandler::new(move |effect: Effect| {
            #[cfg(feature = "gamepad")]
            match (&effect, &pad.input) {
                (Effect::InputOwner(owner), Some(input)) => return input.set_owner(*owner),
                (Effect::NoticeHolds(on), Some(input)) => return input.set_holds(*on),
                _ => {}
            }
            #[cfg(not(feature = "gamepad"))]
            let _ = &pad;
            host.handle(&effect);
        });

        Shell {
            store,
            feed: self.pad.clone(),
            map: self.map.clone(),
            on_effect,
            detected: self.mode.mode,
            dev,
            services: Services::new(WindowHost::attached(self.frame), self.media.clone()),
            start: self.start.clone(),
            script: vec![],
        }
    }
}

/// The page named by `--open <path>`, or the Library.
fn start_route() -> Route {
    let mut args = std::env::args().skip_while(|a| a != "--open");
    args.next();
    match args.next() {
        Some(path) => path.parse().unwrap_or_else(|e| {
            eprintln!("reclaw: {path:?} is not a page ({e}); starting at the Library");
            Route::Library {}
        }),
        None => Route::Library {},
    }
}

fn main() {
    let get = |k: &str| std::env::var(k).ok();
    let mode = detect_environment(get);
    let mut map = ActionMap::default();
    if mode.guide_owner == GuideOwner::Steam {
        map.unbind(Button::Guide);
    }

    let dirs = AppDirs::locate(get);
    let (net, net_problems) = open_net(dirs.as_ref(), get);
    for problem in &net_problems {
        eprintln!("reclaw: {problem}");
    }
    let (store_feed, inbox) = feed();
    let sink: Arc<dyn Sink> = Arc::new(store_feed.clone());
    let library_file = dirs.as_ref().map(AppDirs::library_file).unwrap_or_else(|| std::path::PathBuf::from("apps.json"));
    if dirs.is_none() {
        eprintln!("reclaw: no folder for Reclaw's files was found, so the library is read from and written to ./apps.json");
    }
    let (host, initial) = Host::open(net.clone(), library_file, get("RECLAW_CATALOG_INDEX").filter(|u| !u.is_empty()), sink);

    let opened = open_store(get, |state| {
        state.projects = initial.loaded.projects;
        state.games = initial.loaded.games;
        state.catalog = initial.status;
    });
    if let Some(warning) = &opened.warning {
        eprintln!("reclaw: {warning}");
    }
    // Things to say once the window is up go through the same path as everything else that arrives from another thread.
    for notice in initial
        .notices
        .into_iter()
        .chain(net_problems.into_iter().map(|p| reclaw_ui::notices::Notice::note("Network setting ignored", &p, vec![])))
    {
        store_feed.send(AppAction::Notify(notice));
    }
    let store = opened.store;
    let (_tx, pad) = ActionFeed::new();
    let frame = Frame::from_env(get);
    let server = detect_server(get);
    let media = open_media(opened.dirs.as_ref(), net.as_ref());
    launch(launch_config(
        Launcher { frame, media, store, pad, map, mode, host, inbox: Handoff::new(inbox), start: start_route() },
        store,
        frame,
        server,
    ));
}
