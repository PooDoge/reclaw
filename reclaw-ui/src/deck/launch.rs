//! What the primary button says and does, from install state + run state. One table, used by the
//! button, the reducer and the contract (`lifecycle.ui.LaunchButton`).
use reclaw_runtime::RunState;

use crate::model::AppStatus;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LaunchVerb {
    Install,
    Installing,
    Update,
    Play,
    Starting,
    /// Controller density, app running: bring it back. Stop sits beside it.
    Resume,
    /// Desktop density, app running: the Play button turns into Stop.
    Stop,
    /// Stop was already pressed; pressing again kills.
    ForceQuit,
    Retry,
}

impl LaunchVerb {
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Installing => "Installing",
            Self::Update => "Update",
            Self::Play => "Play",
            Self::Starting => "Starting",
            Self::Resume => "Resume",
            Self::Stop => "Stop",
            Self::ForceQuit => "Force quit",
            Self::Retry => "Retry",
        }
    }

    /// Disabled while there is nothing the press could do.
    pub fn enabled(self) -> bool {
        !matches!(self, Self::Installing | Self::Starting)
    }
}

pub fn launch_verb(status: AppStatus, run: &RunState, controller: bool) -> LaunchVerb {
    match run {
        RunState::Running { .. } => {
            if controller {
                LaunchVerb::Resume
            } else {
                LaunchVerb::Stop
            }
        }
        RunState::Stopping { .. } => LaunchVerb::ForceQuit,
        RunState::Starting => LaunchVerb::Starting,
        RunState::Failed(_) => LaunchVerb::Retry,
        RunState::Idle => match status {
            AppStatus::Installed => LaunchVerb::Play,
            AppStatus::UpdateReady => LaunchVerb::Update,
            AppStatus::Installing => LaunchVerb::Installing,
            AppStatus::Failed => LaunchVerb::Retry,
            AppStatus::Available | AppStatus::NeedsFile => LaunchVerb::Install,
        },
    }
}

/// In controller density a separate Stop button sits beside Resume while the app runs.
pub fn shows_stop_pair(run: &RunState, controller: bool) -> bool {
    controller && matches!(run, RunState::Running { .. })
}

/// Human wording for an unhappy ending, shown under the button.
pub fn failure_message(run: &RunState) -> Option<String> {
    use reclaw_runtime::Outcome::*;
    match run {
        RunState::Failed(ExitedWithCode { code, .. }) => Some(format!("Exited with code {code}. The log has details.")),
        RunState::Failed(Signaled { signal, .. }) => Some(format!("Closed unexpectedly (signal {signal}). The log has details.")),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use reclaw_runtime::Outcome;

    use super::*;

    fn running() -> RunState {
        RunState::Running { pid: 1, since: SystemTime::now() }
    }

    #[test]
    fn idle_installed_plays() {
        assert_eq!(launch_verb(AppStatus::Installed, &RunState::Idle, false), LaunchVerb::Play);
        assert_eq!(launch_verb(AppStatus::UpdateReady, &RunState::Idle, true), LaunchVerb::Update);
        assert_eq!(launch_verb(AppStatus::NeedsFile, &RunState::Idle, true), LaunchVerb::Install);
    }

    #[test]
    fn play_turns_into_stop_on_desktop_and_resume_on_deck() {
        assert_eq!(launch_verb(AppStatus::Installed, &running(), false), LaunchVerb::Stop);
        assert_eq!(launch_verb(AppStatus::Installed, &running(), true), LaunchVerb::Resume);
        assert!(shows_stop_pair(&running(), true));
        assert!(!shows_stop_pair(&running(), false));
    }

    #[test]
    fn stopping_offers_force_quit_and_starting_is_disabled() {
        assert_eq!(launch_verb(AppStatus::Installed, &RunState::Stopping { pid: 1 }, false), LaunchVerb::ForceQuit);
        assert!(!launch_verb(AppStatus::Installed, &RunState::Starting, false).enabled());
        assert!(!launch_verb(AppStatus::Installing, &RunState::Idle, false).enabled());
    }

    #[test]
    fn failure_explains_itself() {
        let f = RunState::Failed(Outcome::ExitedWithCode { code: 3, ran_for: Duration::ZERO });
        assert_eq!(launch_verb(AppStatus::Installed, &f, true), LaunchVerb::Retry);
        assert!(failure_message(&f).unwrap().contains("code 3"));
        assert!(failure_message(&RunState::Idle).is_none());
    }
}
