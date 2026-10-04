//! Shared harness for the integration tests. It mounts the real `Shell` headlessly, so a test
//! drives the same component tree the app runs: key presses in, effects and pixels out.
//!
//! Each test crate uses a subset, hence `allow(dead_code)`.
#![allow(dead_code)]
use std::{
    cell::RefCell,
    path::PathBuf,
    rc::Rc,
    time::{Duration, SystemTime},
};

use freya::prelude::*;
use freya_core::element::AppComponent;
use freya_testing::prelude::*;
use futures_channel::mpsc::UnboundedSender;
use reclaw_input::{Action, ActionMap, ControllerInfo, UiMode};
use reclaw_runtime::RunState;
use reclaw_ui::{
    deck::ActionFeed,
    effect::Effect,
    host::HostState,
    nav::Route,
    prelude::*,
    sample::{sample_downloads, sample_games, sample_mods, sample_projects},
    shell::{DevOverrides, Shell},
};

pub fn out_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("snapshots");
    std::fs::create_dir_all(&dir).expect("create snapshot dir");
    dir
}

pub fn xbox() -> Option<ControllerInfo> {
    Some(ControllerInfo::new(0, "Xbox Wireless Controller", Some(0x045e), Some(0x0b13)))
}

pub fn dualsense() -> Option<ControllerInfo> {
    Some(ControllerInfo::new(1, "DualSense Wireless Controller", Some(0x054c), Some(0x0ce6)))
}

pub fn switch_pro() -> Option<ControllerInfo> {
    Some(ControllerInfo::new(2, "Nintendo Switch Pro Controller", Some(0x057e), Some(0x2009)))
}

pub fn with_run(mut games: Vec<GameEntry>, id: u32, state: RunState) -> Vec<GameEntry> {
    games.iter_mut().find(|g| g.id == id).expect("sample game").run = state;
    games
}

pub fn running() -> RunState {
    RunState::Running { pid: 4242, since: SystemTime::now() - Duration::from_secs(12 * 60) }
}

/// What to mount. Start from [`Mount::deck`] or [`Mount::desktop`] and adjust.
#[derive(Clone)]
pub struct Mount {
    pub size: (f32, f32),
    pub games: Vec<GameEntry>,
    pub pad: Option<ControllerInfo>,
    pub script: Vec<Action>,
    pub mode: UiMode,
    pub dev: DevOverrides,
    pub keyboard: f32,
    pub theme: ThemeKind,
}

impl Mount {
    pub fn deck() -> Self {
        Self {
            size: (1280., 800.),
            games: sample_games(),
            pad: xbox(),
            script: vec![],
            mode: UiMode::Deck,
            dev: DevOverrides::default(),
            keyboard: 0.,
            theme: ThemeKind::Midnight,
        }
    }

    pub fn desktop() -> Self {
        Self { size: (1100., 700.), pad: None, mode: UiMode::Desktop, ..Self::deck() }
    }

    pub fn size(mut self, w: f32, h: f32) -> Self {
        self.size = (w, h);
        self
    }

    pub fn games(mut self, games: Vec<GameEntry>) -> Self {
        self.games = games;
        self
    }

    pub fn pad(mut self, pad: Option<ControllerInfo>) -> Self {
        self.pad = pad;
        self
    }

    pub fn script(mut self, script: impl Into<Vec<Action>>) -> Self {
        self.script = script.into();
        self
    }

    pub fn keyboard(mut self, px: f32) -> Self {
        self.keyboard = px;
        self
    }

    pub fn dev(mut self, dev: DevOverrides) -> Self {
        self.dev = dev;
        self
    }

    pub fn theme(mut self, theme: ThemeKind) -> Self {
        self.theme = theme;
        self
    }

    pub fn start(self) -> Session {
        self.start_at(Route::Library {})
    }

    pub fn start_at(self, start: Route) -> Session {
        let effects: Rc<RefCell<Vec<Effect>>> = Rc::default();
        let stash: Rc<RefCell<Option<HostState>>> = Rc::default();
        let (tx, feed) = ActionFeed::new();
        let Mount { size, games, pad, script, mode, dev, keyboard, theme } = self;

        let app = {
            let (effects, stash) = (effects.clone(), stash.clone());
            move || {
                use_init_reclaw(theme);
                let host = HostState {
                    controller: use_state({
                        let pad = pad.clone();
                        move || pad
                    }),
                    keyboard_inset: use_state(move || keyboard),
                    ..HostState::use_new(games.clone(), sample_downloads(), sample_projects(), sample_mods())
                };
                *stash.borrow_mut() = Some(host);
                let sink = effects.clone();
                Shell {
                    host,
                    feed: feed.clone(),
                    map: ActionMap::default(),
                    on_effect: EventHandler::new(move |e| sink.borrow_mut().push(e)),
                    detected: mode,
                    dev,
                    start: start.clone(),
                    script: script.clone(),
                }
            }
        };
        let (runner, ()) = TestingRunner::new(AppComponent::from(app), size.into(), |_| {}, 1.);
        let mut session = Session { runner, effects, host: stash, feed: tx, size };
        session.settle();
        session
    }
}

pub struct Session {
    pub runner: TestingRunner,
    pub effects: Rc<RefCell<Vec<Effect>>>,
    host: Rc<RefCell<Option<HostState>>>,
    /// Send pad actions here, as the gamepad thread would.
    pub feed: UnboundedSender<Action>,
    pub size: (f32, f32),
}

impl Session {
    fn host(&self) -> HostState {
        self.host.borrow().expect("the app has rendered")
    }

    /// Ask the app to show a page, as the host does for a deep link.
    pub fn open_route(&mut self, route: Route) {
        let host = self.host();
        self.runner.run_in(|| {
            let mut open = host.open;
            open.set(Some(route));
        });
    }

    /// Let layout, effects and the 700 ms slide animations finish.
    pub fn settle(&mut self) {
        for _ in 0..4 {
            self.runner.sync_and_update();
        }
        self.runner.poll(Duration::from_millis(16), Duration::from_millis(700));
        self.runner.sync_and_update();
    }

    /// Run the app for `ms` of simulated time, enough for a spawned task or a process to act.
    pub fn pump(&mut self, ms: u64) {
        self.runner.poll(Duration::from_millis(10), Duration::from_millis(ms));
    }

    pub fn press(&mut self, key: NamedKey) {
        self.runner.press_key(Key::Named(key));
        self.runner.poll(Duration::from_millis(16), Duration::from_millis(120));
        self.runner.sync_and_update();
    }

    pub fn press_char(&mut self, c: &str) {
        self.runner.press_key(Key::Character(c.into()));
        self.runner.poll(Duration::from_millis(16), Duration::from_millis(120));
        self.runner.sync_and_update();
    }

    /// Scroll the wheel by `dy` px (negative moves the content up) with the pointer at `(x, y)`.
    pub fn wheel(&mut self, x: f32, y: f32, dy: f32) {
        self.runner.scroll((f64::from(x), f64::from(y)), (0., f64::from(dy)));
        self.runner.poll(Duration::from_millis(16), Duration::from_millis(200));
        self.runner.sync_and_update();
    }

    pub fn presses(&mut self, keys: &[NamedKey]) {
        for key in keys {
            self.press(*key);
        }
    }

    pub fn type_text(&mut self, text: &str) {
        self.runner.write_text(text);
        self.runner.sync_and_update();
    }

    /// Send a pad action through the feed and let it be applied.
    pub fn pad(&mut self, action: Action) {
        self.feed.unbounded_send(action).expect("feed is open");
        self.pump(120);
        self.runner.sync_and_update();
    }

    /// Move the (simulated) on-screen keyboard, as the OS would.
    pub fn set_keyboard(&mut self, px: f32) {
        let host = self.host();
        self.runner.run_in(|| {
            let mut inset = host.keyboard_inset;
            inset.set(px);
        });
        self.pump(60);
        self.runner.sync_and_update();
    }

    pub fn set_run(&mut self, id: u32, state: RunState) {
        let host = self.host();
        self.runner.run_in(|| {
            let mut games = host.games;
            if let Some(g) = games.write().iter_mut().find(|g| g.id == id) {
                g.run = state;
            }
        });
        self.pump(60);
        self.runner.sync_and_update();
    }

    pub fn effects(&self) -> Vec<Effect> {
        self.effects.borrow().clone()
    }

    pub fn take_effects(&self) -> Vec<Effect> {
        std::mem::take(&mut *self.effects.borrow_mut())
    }

    /// Top and bottom of the first label whose text is exactly `text`, in window coordinates.
    pub fn label_span(&self, text: &str) -> Option<(f32, f32)> {
        self.runner.find(|node, element| {
            Label::try_downcast(element).filter(|l| l.text == text).map(|_| {
                let area = node.layout().area;
                (area.min_y(), area.max_y())
            })
        })
    }

    /// `(left, top, right, bottom)` of the first label whose text is exactly `text`.
    pub fn label_box(&self, text: &str) -> Option<(f32, f32, f32, f32)> {
        self.runner.find(|node, element| {
            Label::try_downcast(element).filter(|l| l.text == text).map(|_| {
                let a = node.layout().area;
                (a.min_x(), a.min_y(), a.max_x(), a.max_y())
            })
        })
    }

    /// Like [`label_box`](Self::label_box) but the last match: overlays are drawn after the page,
    /// so the last label with a given text is the one on top.
    pub fn top_label_box(&self, text: &str) -> Option<(f32, f32, f32, f32)> {
        self.runner
            .find_many(|node, element| {
                Label::try_downcast(element).filter(|l| l.text == text).map(|_| {
                    let a = node.layout().area;
                    (a.min_x(), a.min_y(), a.max_x(), a.max_y())
                })
            })
            .pop()
    }

    /// Click the middle of the topmost label with this text. Panics with the labels on screen if absent.
    pub fn click_label(&mut self, text: &str) {
        let (l, t, r, b) = self.top_label_box(text).unwrap_or_else(|| panic!("no label {text:?} in {:?}", self.labels()));
        self.runner.click_cursor((f64::from((l + r) / 2.), f64::from((t + b) / 2.)));
        self.settle();
    }

    pub fn has_label(&self, text: &str) -> bool {
        self.label_span(text).is_some()
    }

    /// Every label's text, for failure messages.
    pub fn labels(&self) -> Vec<String> {
        self.runner.find_many(|_, element| Label::try_downcast(element).map(|l| l.text.to_string()))
    }

    pub fn snapshot(&mut self, name: &str) {
        self.runner.sync_and_update();
        self.runner.render_to_file(out_dir().join(format!("{name}.png")));
    }
}
