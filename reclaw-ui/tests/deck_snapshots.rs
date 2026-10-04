//! Headless renders of Deck mode scenarios, written to target/snapshots/deck-*.png.
use std::{path::PathBuf, time::SystemTime};

use freya::prelude::*;
use freya_core::element::AppComponent;
use freya_testing::prelude::*;
use reclaw_input::{Action, ActionMap, ControllerInfo, Direction};
use reclaw_runtime::{Outcome, RunState};
use reclaw_ui::{
    deck::{ActionFeed, DeckApp},
    prelude::*,
    sample::{sample_downloads, sample_games},
};

fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("snapshots");
    std::fs::create_dir_all(&dir).expect("create snapshot dir");
    dir
}

struct Scenario {
    name: &'static str,
    size: (f32, f32),
    games: Vec<GameEntry>,
    script: Vec<Action>,
    pad: Option<ControllerInfo>,
}

fn xbox() -> Option<ControllerInfo> {
    Some(ControllerInfo::new(
        0,
        "Xbox Wireless Controller",
        Some(0x045e),
        Some(0x0b13),
    ))
}

fn run(s: Scenario) {
    let Scenario {
        name,
        size,
        games,
        script,
        pad,
    } = s;
    let app = move || {
        use_init_reclaw(ThemeKind::Midnight);
        let games = use_state({
            let g = games.clone();
            move || g
        });
        let downloads = use_state(sample_downloads);
        let controller = use_state({
            let p = pad.clone();
            move || p
        });
        let (_tx, feed) = ActionFeed::new();
        DeckApp {
            games,
            downloads,
            controller,
            feed,
            on_effect: EventHandler::new(|_| {}),
            map: ActionMap::default(),
            script: script.clone(),
        }
    };
    let (mut runner, ()) = TestingRunner::new(AppComponent::from(app), size.into(), |_| {}, 1.);
    for _ in 0..4 {
        runner.sync_and_update();
    }
    // Let slide-in animations finish; the panels mount already open in these scenarios.
    runner.poll(
        std::time::Duration::from_millis(16),
        std::time::Duration::from_millis(700),
    );
    runner.sync_and_update();
    runner.render_to_file(out_dir().join(format!("deck-{name}.png")));
}

fn with_run(mut games: Vec<GameEntry>, id: u32, state: RunState) -> Vec<GameEntry> {
    games.iter_mut().find(|g| g.id == id).unwrap().run = state;
    games
}

fn running() -> RunState {
    RunState::Running {
        pid: 4242,
        since: SystemTime::now() - std::time::Duration::from_secs(12 * 60),
    }
}

use Action::*;
use Direction::*;

#[test]
fn deck_home() {
    run(Scenario {
        name: "home",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![],
        pad: xbox(),
    });
    run(Scenario {
        name: "home-second-shelf",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![Navigate(Right), Navigate(Down), Navigate(Right)],
        pad: xbox(),
    });
    run(Scenario {
        name: "home-tv",
        size: (1920., 1080.),
        games: sample_games(),
        script: vec![],
        pad: xbox(),
    });
}

#[test]
fn deck_with_a_running_app() {
    let games = with_run(sample_games(), 1, running());
    run(Scenario {
        name: "home-running",
        size: (1280., 800.),
        games: games.clone(),
        script: vec![],
        pad: xbox(),
    });
    run(Scenario {
        name: "quick-access-running",
        size: (1280., 800.),
        games: games.clone(),
        script: vec![QuickAccess],
        pad: xbox(),
    });
    run(Scenario {
        name: "game-running",
        size: (1280., 800.),
        games,
        script: vec![Navigate(Down), Confirm],
        pad: xbox(),
    });
}

#[test]
fn deck_overlays_and_pages() {
    run(Scenario {
        name: "main-menu",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![MainMenu, Navigate(Down)],
        pad: xbox(),
    });
    run(Scenario {
        name: "game-installed",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![Confirm],
        pad: xbox(),
    });
    run(Scenario {
        name: "downloads",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![NextSection, NextSection],
        pad: xbox(),
    });
    run(Scenario {
        name: "quick-access-idle",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![QuickAccess],
        pad: None,
    });
}

#[test]
fn deck_states_and_glyph_sets() {
    let stopping = with_run(sample_games(), 1, RunState::Stopping { pid: 1 });
    run(Scenario {
        name: "game-stopping",
        size: (1280., 800.),
        games: stopping,
        script: vec![Confirm],
        pad: xbox(),
    });
    let failed = with_run(
        sample_games(),
        1,
        RunState::Failed(Outcome::ExitedWithCode {
            code: 3,
            ran_for: std::time::Duration::from_secs(90),
        }),
    );
    run(Scenario {
        name: "game-failed",
        size: (1280., 800.),
        games: failed,
        script: vec![Confirm],
        pad: xbox(),
    });
    let ps = Some(ControllerInfo::new(
        1,
        "DualSense Wireless Controller",
        Some(0x054c),
        Some(0x0ce6),
    ));
    run(Scenario {
        name: "home-playstation",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![],
        pad: ps,
    });
    let nintendo = Some(ControllerInfo::new(
        2,
        "Nintendo Switch Pro Controller",
        Some(0x057e),
        Some(0x2009),
    ));
    run(Scenario {
        name: "home-nintendo",
        size: (1280., 800.),
        games: sample_games(),
        script: vec![],
        pad: nintendo,
    });
}

/// Drive the real component with key presses and watch the effects the host receives.
#[test]
fn keyboard_drives_the_reducer_and_reports_effects() {
    use std::{cell::RefCell, rc::Rc};

    use reclaw_ui::deck::Effect;

    let effects: Rc<RefCell<Vec<Effect>>> = Rc::default();
    let sink = effects.clone();
    let app = move || {
        use_init_reclaw(ThemeKind::Midnight);
        let games = use_state(sample_games);
        let downloads = use_state(sample_downloads);
        let controller = use_state(|| None);
        let (_tx, feed) = ActionFeed::new();
        let sink = sink.clone();
        DeckApp {
            games,
            downloads,
            controller,
            feed,
            on_effect: EventHandler::new(move |e| sink.borrow_mut().push(e)),
            map: ActionMap::default(),
            script: vec![],
        }
    };
    let (mut runner, ()) =
        TestingRunner::new(AppComponent::from(app), (1280., 800.).into(), |_| {}, 1.);
    for _ in 0..3 {
        runner.sync_and_update();
    }
    let mut press = |key: Key| {
        runner.press_key(key);
        runner.sync_and_update();
    };
    press(Key::Named(NamedKey::Enter)); // open Starfall 64
    press(Key::Named(NamedKey::Enter)); // Play
    assert_eq!(
        *effects.borrow(),
        vec![Effect::Launch(1)],
        "Play launches the focused, installed game"
    );

    press(Key::Named(NamedKey::Escape)); // back to Home
    press(Key::Named(NamedKey::Tab)); // main menu
    press(Key::Named(NamedKey::Escape)); // close it
    assert_eq!(
        effects.borrow().len(),
        1,
        "navigation alone produces no effects"
    );
}
