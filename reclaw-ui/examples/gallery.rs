//! `cargo run -p reclaw-ui --example gallery`
//!
//! The whole UI with sample data and no gamepad or processes, for looking at layouts. Starts in the
//! desktop interface unless the environment says otherwise.
//!
//! * **F10** switches between the desktop and Deck mode; **F9** shows a simulated on-screen keyboard.
//! * Resize the window across 720px and 1100px to see the three desktop layout classes.
//! * Environment: `RECLAW_MODE=deck|desktop`, `RECLAW_LAYOUT=wide|compact|phone`,
//!   `RECLAW_DENSITY=pointer|touch|controller`, `RECLAW_THEME=midnight|daylight`,
//!   `RECLAW_KEYBOARD=auto|<px>` (start with the keyboard up), `RECLAW_SIM_KEYBOARD=1` (raise it
//!   whenever a text box takes focus).
use freya::prelude::*;
use reclaw_input::{ActionMap, detect_environment};
use reclaw_ui::{
    deck::ActionFeed,
    host::HostState,
    nav::Route,
    prelude::*,
    sample::{sample_downloads, sample_games, sample_mods, sample_projects},
    shell::{DevOverrides, Shell},
};

#[derive(Clone, PartialEq)]
struct Gallery {
    feed: ActionFeed,
}

impl App for Gallery {
    fn render(&self) -> impl IntoElement {
        let get = |k: &str| std::env::var(k).ok();
        let dev = DevOverrides::from_env(get);
        use_init_reclaw(dev.theme.unwrap_or(ThemeKind::Midnight));
        let host = HostState::use_new(sample_games(), sample_downloads(), sample_projects(), sample_mods());
        Shell {
            host,
            feed: self.feed.clone(),
            map: ActionMap::default(),
            // No host behind the gallery: show what the UI asked for.
            on_effect: EventHandler::new(|effect| eprintln!("effect: {effect:?}")),
            detected: detect_environment(get).mode,
            dev,
            start: Route::Library {},
            script: vec![],
        }
    }
}

fn main() {
    // The gallery has no pad, so the feed's sender is dropped and it simply stays quiet.
    let (_tx, feed) = ActionFeed::new();
    launch(
        LaunchConfig::new()
            .with_window(WindowConfig::new_app(Gallery { feed }).with_title("Reclaw").with_size(1100., 700.).with_min_size(360., 480.)),
    );
}
