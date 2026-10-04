//! End to end: key presses drive the real DeckApp, which launches a real process through the real
//! Supervisor, hands the pad to it, takes it back on Guide, and stops it. Unix only (uses `sh`).
#![cfg(unix)]
use std::{cell::RefCell, rc::Rc, time::Duration};

use freya::prelude::*;
use freya_core::element::AppComponent;
use freya_testing::prelude::*;
use futures_util::StreamExt;
use reclaw_input::{ActionMap, InputOwner};
use reclaw_runtime::{LaunchSpec, RunState, SessionEvent, Supervisor};
use reclaw_ui::{
    deck::{ActionFeed, DeckApp},
    effect::Effect,
    host::HostState,
    prelude::*,
    sample::{sample_downloads, sample_games},
};

/// Kills whatever the test started, even when an assertion fails halfway: the second `stop` is the
/// supervisor's force-kill. Without it a failed run leaves a shell loop behind.
struct KillOnDrop(Supervisor);

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        for app in self.0.running() {
            let _ = self.0.stop(app);
            let _ = self.0.stop(app);
        }
    }
}

fn pump(runner: &mut TestingRunner, ms: u64) {
    runner.poll(Duration::from_millis(10), Duration::from_millis(ms));
}

#[test]
fn play_guide_resume_stop_with_a_real_process() {
    let log: Rc<RefCell<Vec<Effect>>> = Rc::default();
    let (supervisor, events) = Supervisor::new(Duration::from_secs(2));
    let _cleanup = KillOnDrop(supervisor.clone());
    let events = Rc::new(RefCell::new(Some(events)));

    let app = {
        let (log, supervisor, events) = (log.clone(), supervisor.clone(), events.clone());
        move || {
            use_init_reclaw(ThemeKind::Midnight);
            let host = HostState::use_new(sample_games(), sample_downloads());
            let mut games = host.games;
            let (_tx, feed) = ActionFeed::new();

            use_hook({
                let (supervisor, events) = (supervisor.clone(), events.clone());
                move || {
                    spawn(async move {
                        let Some(mut rx) = events.borrow_mut().take() else { return };
                        while let Some(event) = rx.next().await {
                            let (SessionEvent::Started { app, .. } | SessionEvent::Ended { app, .. }) = event;
                            if let Some(game) = games.write().iter_mut().find(|g| g.id == app) {
                                game.run = supervisor.state(app);
                            }
                        }
                    })
                }
            });

            let (log, supervisor) = (log.clone(), supervisor.clone());
            DeckApp {
                host,
                feed,
                on_effect: EventHandler::new(move |effect: Effect| {
                    log.borrow_mut().push(effect.clone());
                    match effect {
                        Effect::Launch(id) => {
                            let spec = LaunchSpec::new("sh").arg("-c").arg("while true; do sleep 1; done");
                            supervisor.start(id, spec).expect("launch");
                            if let Some(g) = games.write().iter_mut().find(|g| g.id == id) {
                                g.run = supervisor.state(id);
                            }
                        }
                        Effect::Stop(id) => {
                            supervisor.stop(id).expect("stop");
                            if let Some(g) = games.write().iter_mut().find(|g| g.id == id) {
                                g.run = supervisor.state(id);
                            }
                        }
                        _ => {}
                    }
                }),
                map: ActionMap::default(),
                script: vec![],
            }
        }
    };

    let (mut runner, ()) = TestingRunner::new(AppComponent::from(app), (1280., 800.).into(), |_| {}, 1.);
    pump(&mut runner, 100);
    let press = |runner: &mut TestingRunner, key: NamedKey| {
        runner.press_key(Key::Named(key));
        pump(runner, 120);
    };

    // Open the game page and press Play.
    press(&mut runner, NamedKey::Enter);
    press(&mut runner, NamedKey::Enter);
    assert!(matches!(supervisor.state(1), RunState::Running { .. }), "the process is really running");
    assert_eq!(*log.borrow(), vec![Effect::Launch(1), Effect::InputOwner(InputOwner::App)], "the pad goes to the app");

    // Guide (Tab on a keyboard) brings the launcher forward and takes the pad back.
    log.borrow_mut().clear();
    press(&mut runner, NamedKey::Tab);
    assert_eq!(*log.borrow(), vec![Effect::BringLauncherToFront, Effect::InputOwner(InputOwner::Launcher)]);

    // Back closes the menu and hands the pad to the app again.
    log.borrow_mut().clear();
    press(&mut runner, NamedKey::Escape);
    assert_eq!(*log.borrow(), vec![Effect::SendLauncherToBack, Effect::InputOwner(InputOwner::App)]);

    // Stop from the game page: the page shows Resume + Stop; Right moves to Stop.
    log.borrow_mut().clear();
    press(&mut runner, NamedKey::Tab);
    press(&mut runner, NamedKey::Escape);
    log.borrow_mut().clear();
    press(&mut runner, NamedKey::ArrowRight);
    press(&mut runner, NamedKey::Enter);
    assert!(log.borrow().contains(&Effect::Stop(1)), "Stop reaches the host: {:?}", log.borrow());

    // The supervisor ends the session; the app notices and brings the launcher forward.
    pump(&mut runner, 1500);
    assert_eq!(supervisor.state(1), RunState::Idle, "SIGTERM ended the whole group");
    assert!(log.borrow().contains(&Effect::BringLauncherToFront), "the launcher comes forward when the app ends: {:?}", log.borrow());
    assert_eq!(log.borrow().last(), Some(&Effect::InputOwner(InputOwner::Launcher)));
}
