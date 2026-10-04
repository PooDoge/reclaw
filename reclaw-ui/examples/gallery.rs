//! `cargo run -p reclaw-ui --example gallery`
//!
//! The whole UI with sample data and no gamepad or processes, for looking at layouts. Starts in the
//! desktop interface unless the environment says otherwise. Settings and favorites are saved like
//! the real app's; `RECLAW_HOME=/tmp/reclaw-try` keeps them out of your real profile.
//!
//! * **F10** switches between the desktop and Deck mode; **F9** shows a simulated on-screen keyboard.
//! * Resize the window across 720px and 1100px to see the three desktop layout classes.
//! * `--open /game/4` starts on that page (any route path works).
//! * Environment: `RECLAW_MODE=deck|desktop`, `RECLAW_LAYOUT=wide|compact|phone`,
//!   `RECLAW_DENSITY=pointer|touch|controller`, `RECLAW_THEME=midnight|daylight`,
//!   `RECLAW_MOTION=reduced|subtle|standard|cinematic`, `RECLAW_KEYBOARD=auto|<px>` (start with the
//!   keyboard up), `RECLAW_SIM_KEYBOARD=1` (raise it whenever a text box takes focus),
//!   `RECLAW_WINDOW_FRAME=native` (the window manager's border instead of Reclaw's own title bar).
use freya::prelude::*;
use reclaw_input::{ActionMap, detect_environment};
use reclaw_ui::{
    bootstrap::{open_media, open_store},
    deck::ActionFeed,
    nav::Route,
    prelude::*,
    shell::{DevOverrides, Services, Shell},
    store::{AppState, Store},
    window::{Frame, WindowHost, detect_server, launch::launch_config},
};

#[derive(Clone, PartialEq)]
struct Gallery {
    feed: ActionFeed,
    store: Store,
    start: Route,
    frame: Frame,
    media: Option<reclaw_media::MediaHub>,
}

impl App for Gallery {
    fn render(&self) -> impl IntoElement {
        let get = |k: &str| std::env::var(k).ok();
        let dev = DevOverrides::from_env(get);
        use_init_reclaw(dev.theme.unwrap_or_else(|| self.store.with(|s| ThemeKind::from_settings(&s.settings))));
        Shell {
            store: self.store,
            feed: self.feed.clone(),
            map: ActionMap::default(),
            // No host behind the gallery: show what the UI asked for.
            on_effect: EventHandler::new(|effect| eprintln!("effect: {effect:?}")),
            detected: detect_environment(get).mode,
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
            eprintln!("gallery: {path:?} is not a page ({e}); starting at the Library");
            Route::Library {}
        }),
        None => Route::Library {},
    }
}

fn main() {
    let opened = open_store(
        |k| std::env::var(k).ok(),
        |state| {
            let sample = AppState::sample();
            state.games = sample.games;
            state.projects = sample.projects;
            state.mods = sample.mods;
            state.activity = sample.activity;
        },
    );
    if let Some(warning) = &opened.warning {
        eprintln!("gallery: {warning}");
    }
    let store = opened.store;
    // The gallery has no pad, so the feed's sender is dropped and it simply stays quiet.
    let (_tx, feed) = ActionFeed::new();
    // RECLAW_WINDOW_FRAME=native keeps the window manager's border and title bar.
    let frame = Frame::from_env(|k| std::env::var(k).ok());
    let server = detect_server(|k| std::env::var(k).ok());
    let media = open_media(opened.dirs.as_ref(), |k| std::env::var(k).ok());
    launch(launch_config(Gallery { feed, store, start: start_route(), frame, media }, store, frame, server));
}
