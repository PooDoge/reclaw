use std::time::SystemTime;

pub(super) use reclaw_input::{Action, Direction, FocusId, InputOwner};
pub(super) use reclaw_runtime::RunState;

pub(super) use crate::deck::state::*;
use reclaw_config::LaunchPrefs;
use reclaw_games::{project::ProjectInfo, settings::DisplayEnvironment};

use crate::{
    activity::Activity,
    fixtures::{sample_activity, sample_games, sample_projects},
    model::GameEntry,
    notices::Notices,
    settings::LaunchContext,
};

pub(super) fn running() -> RunState {
    RunState::Running { pid: 7, since: SystemTime::now() }
}

/// The sample library and queue, with helpers to build a view and change an app's run state.
pub(super) struct Fixture {
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Activity>,
    pub projects: Vec<ProjectInfo>,
    pub env: DisplayEnvironment,
    pub launch: LaunchPrefs,
    pub notices: Notices,
    pub recents: Vec<crate::nav::Route>,
}

impl Fixture {
    pub fn new() -> Self {
        Self {
            games: sample_games(),
            downloads: sample_activity().queue().into_iter().cloned().collect(),
            projects: sample_projects(),
            env: DisplayEnvironment::unknown(),
            launch: LaunchPrefs::default(),
            notices: Notices::default(),
            recents: Vec::new(),
        }
    }

    pub fn view(&self) -> DeckView<'_> {
        DeckView {
            games: &self.games,
            downloads: &self.downloads,
            launch: Some(LaunchContext { env: &self.env, projects: &self.projects, prefs: &self.launch }),
            notices: Some(&self.notices),
            sort: crate::systems::Sort::default(),
            recents: &self.recents,
        }
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

/// Apply a sequence of actions, returning every effect for the host in order. Requests to the router
/// are left out: most tests are about what the host is asked, and `press_with_nav` keeps them.
pub(super) fn press(s: &mut DeckState, f: &Fixture, actions: &[Action]) -> Vec<Effect> {
    press_with_nav(s, f, actions).into_iter().filter(|e| !matches!(e, Effect::Navigate(_) | Effect::Back)).collect()
}

/// Like [`press`], with the requests to the router included.
pub(super) fn press_with_nav(s: &mut DeckState, f: &Fixture, actions: &[Action]) -> Vec<Effect> {
    actions.iter().flat_map(|a| s.apply(*a, &f.view())).collect()
}

pub(super) use Action::*;
pub(super) use Direction::*;

pub(super) fn go(dir: Direction) -> Action {
    Action::Navigate(dir)
}
