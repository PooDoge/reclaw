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
    deck::ActionFeed,
    effect::Effect,
    host::HostState,
    prelude::*,
    sample::{sample_downloads, sample_games},
    shell::{DevOverrides, Shell},
};

#[derive(Clone, PartialEq)]
struct Host {
    feed: ActionFeed,
    map: ActionMap,
    env: reclaw_input::Environment,
}

impl App for Host {
    fn render(&self) -> impl IntoElement {
        let dev = DevOverrides::from_env(|k| std::env::var(k).ok());
        use_init_reclaw(dev.theme.unwrap_or(ThemeKind::Midnight));
        let host = HostState::use_new(sample_games(), sample_downloads());
        let (mut games, mut controller) = (host.games, host.controller);

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
                            InputMessage::Connected(info) => controller.set(Some(info)),
                            InputMessage::Disconnected { .. } => controller.set(None),
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
                        if let Some(game) = games.write().iter_mut().find(|g| g.id == app) {
                            game.run = watched.state(app);
                        }
                    }
                });
                (supervisor, input)
            }
        });

        let on_effect = EventHandler::new(move |effect: Effect| match effect {
            Effect::Launch(id) => {
                set_run(&mut games, id, RunState::Starting);
                let mut spec = LaunchSpec::new("sh").arg("-c").arg("echo started; while true; do sleep 1; done");
                InputProfile { allow_background_events: true, ..Default::default() }.apply(&mut spec);
                match supervisor.start(id, spec) {
                    Ok(_) => set_run(&mut games, id, supervisor.state(id)),
                    Err(e) => {
                        eprintln!("launch failed: {e}");
                        set_run(&mut games, id, RunState::Idle);
                    }
                }
            }
            // Stop twice to force: the supervisor treats the second call as SIGKILL.
            Effect::Stop(id) => {
                if supervisor.stop(id).is_ok() {
                    set_run(&mut games, id, supervisor.state(id));
                }
            }
            Effect::InputOwner(owner) => input.set_owner(owner),
            // The rest need real windows, a catalog, or an installer: the host app's job.
            other => eprintln!("effect for the host to handle: {other:?}"),
        });

        Shell { host, feed: self.feed.clone(), map: self.map.clone(), on_effect, detected: self.env.mode, dev, script: vec![] }
    }
}

fn set_run(games: &mut State<Vec<GameEntry>>, id: u32, run: RunState) {
    if let Some(game) = games.write().iter_mut().find(|g| g.id == id) {
        game.run = run;
    }
}

fn main() {
    let env = detect_environment(|k| std::env::var(k).ok());
    eprintln!("deck example: {:?} ({})", env.mode, env.reason);
    let mut map = ActionMap::default();
    if env.guide_owner == GuideOwner::Steam {
        map.unbind(Button::Guide);
    }
    let (_tx, feed) = ActionFeed::new();
    launch(
        LaunchConfig::new().with_window(
            WindowConfig::new_app(Host { feed, map, env }).with_title("Reclaw").with_size(1280., 800.).with_min_size(640., 400.),
        ),
    );
}
