//! What the host needs to start an app the way its person set it up: the launch settings of the game and the defaults turned into
//! arguments, variables and config edits (`reclaw_games::settings::plan`), and the free-text "launch options" row. Worked out from
//! the shared state, so the program's entry point can attach it to a Play press without the host knowing about the store.
use reclaw_games::settings::{LaunchPlan, SettingsLayer, plan};

use crate::{
    settings::{SettingsTarget, SettingsValues},
    store::AppState,
};

/// The key of the per-app "Launch options" text.
pub const KEY_LAUNCH_OPTIONS: &str = "launch_options";

#[derive(Clone, Default, PartialEq, Debug)]
pub struct LaunchRequest {
    /// From the game's capabilities and the person's choices: appended after the game's own arguments.
    pub plan: LaunchPlan,
    /// The words of the "Launch options" row, after the plan's arguments.
    pub options: Vec<String>,
    /// Something in the request that could not be used (a quote left open in the options), to say once.
    pub problem: Option<String>,
}

/// The words of a launch-options line. An unmatched quote is a mistake to report, not a reason to guess.
pub fn split_options(text: &str) -> Result<Vec<String>, String> {
    reclaw_runtime::split_command(text).map_err(|why| format!("The launch options have {why}, so they were not used."))
}

/// The request for app `id`.
pub fn request_for(state: &AppState, id: u32) -> LaunchRequest {
    let caps = state.projects.iter().find(|p| p.id == id).map(|p| p.capabilities.clone()).unwrap_or_default();
    let overrides = state.launch.apps.get(&id).cloned().unwrap_or_else(SettingsLayer::default);
    let plan = plan(&caps, &state.display, &state.launch.defaults, &overrides);
    let (options, problem) = options_of(&state.settings, id);
    LaunchRequest { plan, options, problem }
}

fn options_of(settings: &SettingsValues, id: u32) -> (Vec<String>, Option<String>) {
    match settings.text(SettingsTarget::App(id), KEY_LAUNCH_OPTIONS).map(str::trim).filter(|t| !t.is_empty()) {
        None => (Vec::new(), None),
        Some(text) => match split_options(text) {
            Ok(words) => (words, None),
            Err(problem) => (Vec::new(), Some(problem)),
        },
    }
}

#[cfg(test)]
mod tests {
    use reclaw_games::settings::{SettingKey, SettingValue};

    use super::*;

    fn state_with(options: Option<&str>) -> AppState {
        let mut state = AppState::default();
        if let Some(text) = options {
            state.settings.set_text(SettingsTarget::App(1), KEY_LAUNCH_OPTIONS, text.to_string());
        }
        state
    }

    #[test]
    fn nothing_set_is_an_empty_request() {
        assert_eq!(request_for(&state_with(None), 1), LaunchRequest::default());
    }

    #[test]
    fn the_options_row_becomes_words_with_quotes_respected() {
        let request = request_for(&state_with(Some(r#"--fullscreen --name "My Save" -v"#)), 1);
        assert_eq!(request.options, ["--fullscreen", "--name", "My Save", "-v"]);
        assert_eq!(request.problem, None);
    }

    #[test]
    fn another_apps_options_are_not_this_apps() {
        assert!(request_for(&state_with(Some("--x")), 2).options.is_empty());
    }

    #[test]
    fn a_quote_left_open_is_reported_and_nothing_is_guessed() {
        let request = request_for(&state_with(Some(r#"--name "oops"#)), 1);
        assert!(request.options.is_empty());
        assert!(request.problem.is_some_and(|p| p.contains("launch options") && p.contains("not used")));
    }

    #[test]
    fn a_game_that_declares_nothing_gets_no_plan_whatever_the_defaults_say() {
        let mut state = state_with(None);
        state.launch.defaults.set(SettingKey::Vsync, SettingValue::Bool(true));
        assert_eq!(request_for(&state, 1).plan, LaunchPlan::default());
    }
}
