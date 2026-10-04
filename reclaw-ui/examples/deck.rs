//! `cargo run -p reclaw-ui --example deck --features gamepad`
//!
//! The app shell wired to a real gamepad reader and the process supervisor. "Games" here are a
//! shell loop standing in for a recompiled game, so Play, Resume, Stop and Force quit are real
//! processes. Arrow keys, Enter, Esc, Tab and Shift+Tab work without a pad.
//!
//! RECLAW_MODE / Steam variables pick the starting interface (under Steam the Guide button is left
//! to Steam). F10 switches interface by hand; F9 shows a simulated on-screen keyboard.
use std::time::Duration;

use freya::prelude::*;
use futures_util::StreamExt;
use reclaw_input::{
    ActionMap, Button, GuideOwner,
    backend::{self, InputMessage},
    detect_environment,
};
use reclaw_runtime::{InputProfile, LaunchSpec, RunState, SessionEvent, Supervisor};
use reclaw_ui::{
    bootstrap::open_store,
    deck::ActionFeed,
    effect::Effect,
    nav::Route,
    prelude::*,
    shell::{DevOverrides, Shell},
    store::{AppAction, AppState, Store},
};

#[derive(Clone, PartialEq)]
struct Host {
    store: Store,
    feed: ActionFeed,
    map: ActionMap,
    env: reclaw_input::Environment,
}

impl App for Host {
    fn render(&self) -> impl IntoElement {
        let dev = DevOverrides::from_env(|k| std::env::var(k).ok());
        use_init_reclaw(dev.theme.unwrap_or(ThemeKind::Midnight));
        let store = self.store;

        // Everything long-lived is created once.
        let (supervisor, input) = use_hook({
            let map = self.map.clone();
            let feed_tx = self.feed.clone();
            move || {
                let (supervisor, mut events) = Supervisor::new(Duration::from_secs(5));
                let (input, mut messages) = backend::spawn(map);

                // Gamepad thread -> UI: actions go to the feed's sender, controller changes to state.
                let tx = feed_tx.sender();
                spawn(async move {
                    while let Some(message) = messages.next().await {
                        match message {
                            InputMessage::Action(a) => {
                                let _ = tx.unbounded_send(a);
                            }
                            InputMessage::Connected(info) => store.dispatch(AppAction::SetController(Some(info))),
                            InputMessage::Disconnected { .. } => store.dispatch(AppAction::SetController(None)),
                            InputMessage::Unavailable(why) => {
                                eprintln!("gamepad unavailable: {why}")
                            }
                        }
                    }
                });

                // Supervisor -> UI: mirror each session's state into the game list.
                let watched = supervisor.clone();
                spawn(async move {
                    while let Some(event) = events.next().await {
                        let app = match event {
                            SessionEvent::Started { app, .. } | SessionEvent::Ended { app, .. } => app,
                        };
                        store.dispatch(AppAction::SetRun { id: app, run: watched.state(app) });
                    }
                });
                (supervisor, input)
            }
        });

        let on_effect = EventHandler::new(move |effect: Effect| match effect {
            Effect::Launch(id) => {
                set_run(store, id, RunState::Starting);
                let mut spec = LaunchSpec::new("sh").arg("-c").arg("echo started; while true; do sleep 1; done");
                InputProfile { allow_background_events: true, ..Default::default() }.apply(&mut spec);
                match supervisor.start(id, spec) {
                    Ok(_) => set_run(store, id, supervisor.state(id)),
                    Err(e) => {
                        eprintln!("launch failed: {e}");
                        set_run(store, id, RunState::Idle);
                    }
                }
            }
            // Stop twice to force: the supervisor treats the second call as SIGKILL.
            Effect::Stop(id) => {
                if supervisor.stop(id).is_ok() {
                    set_run(store, id, supervisor.state(id));
                }
            }
            Effect::InputOwner(owner) => input.set_owner(owner),
            // The rest need real windows, a catalog, or an installer: the host app's job.
            other => eprintln!("effect for the host to handle: {other:?}"),
        });

        Shell {
            store,
            feed: self.feed.clone(),
            map: self.map.clone(),
            on_effect,
            detected: self.env.mode,
            dev,
            start: Route::Library {},
            script: vec![],
        }
    }
}

fn set_run(store: Store, id: u32, run: RunState) {
    store.dispatch(AppAction::SetRun { id, run });
}

fn main() {
    let env = detect_environment(|k| std::env::var(k).ok());
    eprintln!("deck example: {:?} ({})", env.mode, env.reason);
    let mut map = ActionMap::default();
    if env.guide_owner == GuideOwner::Steam {
        map.unbind(Button::Guide);
    }
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
        eprintln!("deck example: {warning}");
    }
    let store = opened.store;
    let (_tx, feed) = ActionFeed::new();
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(Host { store, feed, map, env })
                .with_title("Reclaw")
                .with_size(1280., 800.)
                .with_min_size(640., 400.)
                .with_on_close(move |_, _| {
                    store.flush();
                    CloseDecision::Close
                }),
        ),
    );
}
