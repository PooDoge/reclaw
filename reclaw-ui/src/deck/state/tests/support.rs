use std::time::SystemTime;

pub(super) use reclaw_input::{Action, Direction, FocusId, InputOwner};
pub(super) use reclaw_runtime::RunState;

pub(super) use crate::deck::state::*;
use crate::{
    activity::Activity,
    model::GameEntry,
    sample::{sample_activity, sample_games},
};

pub(super) fn running() -> RunState {
    RunState::Running { pid: 7, since: SystemTime::now() }
}

/// The sample library and queue, with helpers to build a view and change an app's run state.
pub(super) struct Fixture {
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Activity>,
}

impl Fixture {
    pub fn new() -> Self {
        Self { games: sample_games(), downloads: sample_activity().queue().into_iter().cloned().collect() }
    }

    pub fn view(&self) -> DeckView<'_> {
        DeckView { games: &self.games, downloads: &self.downloads }
    }

    pub fn set_run(&mut self, id: u32, run: RunState) {
        self.games.iter_mut().find(|g| g.id == id).unwrap().run = run;
    }

    pub fn state(&self) -> DeckState {
        DeckState::new(&self.view())
    }

    /// A state that has launched game 1 and handed the pad to it.
    pub fn with_app_running(&mut self) -> DeckState {
        self.set_run(1, running());
        let mut s = self.state();
        s.in_front = false;
        s.owner = InputOwner::App;
        s
    }
}

pub(super) fn tile(shelf: usize, game: u32) -> FocusId {
    ids::tile(shelf, game)
}

/// Apply a sequence of actions, returning every effect in order.
pub(super) fn press(s: &mut DeckState, f: &Fixture, actions: &[Action]) -> Vec<Effect> {
    actions.iter().flat_map(|a| s.apply(*a, &f.view())).collect()
}

pub(super) use Action::*;
pub(super) use Direction::*;

pub(super) fn go(dir: Direction) -> Action {
    Action::Navigate(dir)
}
