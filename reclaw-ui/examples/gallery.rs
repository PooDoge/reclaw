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
//!   keyboard up), `RECLAW_SIM_KEYBOARD=1` (raise it whenever a text box takes focus).
use freya::prelude::*;
use reclaw_input::{ActionMap, detect_environment};
use reclaw_ui::{
    bootstrap::open_store,
    deck::ActionFeed,
    nav::Route,
    prelude::*,
    shell::{DevOverrides, Shell},
    store::{AppState, Store},
};

#[derive(Clone, PartialEq)]
struct Gallery {
    feed: ActionFeed,
    store: Store,
    start: Route,
}

impl App for Gallery {
    fn render(&self) -> impl IntoElement {
        let get = |k: &str| std::env::var(k).ok();
        let dev = DevOverrides::from_env(get);
        use_init_reclaw(dev.theme.unwrap_or(ThemeKind::Midnight));
        Shell {
            store: self.store,
            feed: self.feed.clone(),
            map: ActionMap::default(),
            // No host behind the gallery: show what the UI asked for.
            on_effect: EventHandler::new(|effect| eprintln!("effect: {effect:?}")),
            detected: detect_environment(get).mode,
            dev,
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
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(Gallery { feed, store, start: start_route() })
                .with_title("Reclaw")
                .with_size(1100., 700.)
                .with_min_size(360., 480.)
                .with_on_close(move |_, _| {
                    store.flush();
                    CloseDecision::Close
                }),
        ),
    );
}
