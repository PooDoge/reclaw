//! `reclaw`: the launcher. It opens the settings, starts the network layer, shows what the last run saved of the catalog and
//! the library at once, refreshes the catalog in the background, and then runs the window. Everything it does for the screens is
//! in `host.rs`; this file only wires things together.
//!
//! Environment (see `docs/BUILDING.md`): `RECLAW_HOME` (where its folders are), `RECLAW_MODE=deck|desktop`,
//! `RECLAW_CATALOG_INDEX=<url>` (another catalog index), `RECLAW_WINDOW_FRAME=native`, `SSL_CERT_FILE`, `HTTPS_PROXY`,
//! `RECLAW_PROXY`, `GITHUB_TOKEN`, `RECLAW_LOG` (a log filter such as `reclaw_net=debug,warn`). `--open /game/4` starts on a page.
//! F10 switches interface, F9 simulates the on-screen keyboard.
//!
//! Logging starts first: a failure while starting is then in the log too. The messages go to `reclaw.log` in the logs folder
//! (Settings, Diagnostics, opens it), and to the terminal when there is one.
use std::{
    io::IsTerminal,
    sync::{Arc, Mutex},
};

use freya::prelude::*;
use reclaw_app::{EnvToken, Host, HostConfig, Sink, UpdateSource};
use reclaw_config::AppDirs;
use reclaw_input::{ActionMap, Button, GuideOwner, detect_environment};
use reclaw_log::{LogConfig, LogLevel};
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
            tracing::warn!(error = %e, "the gamepad reader did not start; the keyboard still works");
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
                InputMessage::Unavailable(why) => tracing::warn!(%why, "the gamepad is unavailable"),
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
                host.check_tokens();
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
            // Play carries the person's launch settings for that app, worked out from the shared state.
            if let Effect::Launch(app) = &effect {
                let request = store.with(|state| reclaw_ui::launch_request::request_for(state, *app));
                host.launch(*app, &request);
                return;
            }
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
            tracing::warn!(%path, error = %e, "--open was not given a page; starting at the Library");
            Route::Library {}
        }),
        None => Route::Library {},
    }
}

fn main() {
    let get = |k: &str| std::env::var(k).ok();
    let dirs = AppDirs::locate(get);
    // The log first, so that everything after it can say what went wrong. The level is the default until the settings are read.
    let logging = Arc::new(reclaw_log::init(LogConfig {
        dir: dirs.as_ref().map(|d| d.logs.clone()),
        level: LogLevel::default(),
        filter: get("RECLAW_LOG"),
        stderr: std::io::stderr().is_terminal(),
    }));
    // What the About page shows, so that after an update it is plain whether the new build is the one running.
    reclaw_ui::about::set_build(reclaw_ui::about::BuildInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("RECLAW_GIT_SHA").to_string(),
        profile: env!("RECLAW_PROFILE").to_string(),
    });
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        commit = env!("RECLAW_GIT_SHA"),
        profile = env!("RECLAW_PROFILE"),
        os = std::env::consts::OS,
        arch = std::env::consts::ARCH,
        log = ?logging.file(),
        "Reclaw starting"
    );
    let mode = detect_environment(get);
    tracing::info!(interface = ?mode.mode, guide_owner = ?mode.guide_owner, "environment detected");
    let mut map = ActionMap::default();
    if mode.guide_owner == GuideOwner::Steam {
        map.unbind(Button::Guide);
    }

    let (net, net_problems) = open_net(dirs.as_ref(), get);
    for problem in &net_problems {
        tracing::warn!(%problem, "a network setting was ignored");
    }
    let (store_feed, inbox) = feed();
    let sink: Arc<dyn Sink> = Arc::new(store_feed.clone());
    let library_file = dirs.as_ref().map(AppDirs::library_file).unwrap_or_else(|| std::path::PathBuf::from("apps.json"));
    if dirs.is_none() {
        tracing::warn!("no folder for Reclaw's files was found, so the library is read from and written to ./apps.json");
    }
    let (host, initial) = Host::open(
        HostConfig {
            index_url: get("RECLAW_CATALOG_INDEX").filter(|u| !u.is_empty()),
            // Quiver's own override for its catalog API, honoured the same way.
            site_api: get("QUIVER_API").filter(|u| !u.is_empty()),
            secrets_file: dirs.as_ref().map(AppDirs::secrets_file),
            logs_dir: dirs.as_ref().map(|d| d.logs.clone()),
            logging: Some(logging.clone()),
            env_tokens: EnvToken::from_env(get),
            update: UpdateSource::detect(env!("RECLAW_SOURCE_DIR"), env!("RECLAW_PROFILE")),
            downloads_dir: dirs.as_ref().map(AppDirs::downloads_dir),
            default_location: reclaw_ui::bootstrap::stored_default_location(dirs.as_ref()),
            home: get("HOME").or_else(|| get("USERPROFILE")).filter(|h| !h.is_empty()).map(std::path::PathBuf::from),
            protect: dirs.iter().flat_map(|d| [d.config.clone(), d.data.clone(), d.cache.clone(), d.logs.clone()]).collect(),
            ..HostConfig::new(net.clone(), library_file)
        },
        sink,
    );

    let opened = open_store(get, |state| {
        state.projects = initial.loaded.projects;
        state.games = initial.loaded.games;
        state.catalog = initial.status;
        state.credentials = initial.credentials;
        state.mods = initial.mods;
    });
    if let Some(warning) = &opened.warning {
        tracing::warn!(%warning, "the settings file was not used as it was");
    }
    // The level the person chose in Settings, now that the settings are read.
    let chosen = opened.store.snapshot().settings.choice(
        reclaw_ui::settings::SettingsTarget::Global,
        reclaw_ui::settings::KEY_LOG_LEVEL,
        LogLevel::default().index(),
    );
    logging.set_level(LogLevel::from_index(chosen));
    // Things to say once the window is up go through the same path as everything else that arrives from another thread.
    let startup_problems =
        logging.problems().iter().chain(net_problems.iter()).map(|p| reclaw_ui::notices::Notice::note("Startup note", p, vec![]));
    for notice in initial.notices.into_iter().chain(startup_problems) {
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
