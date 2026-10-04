//! What the settings say about the window, as plain decisions the driver carries out.
use reclaw_input::UiMode;

use super::command::WindowCommand;
use crate::settings::{KEY_DECK_DISPLAY, KEY_DECK_FULLSCREEN, KEY_UI_SCALE, SCALE_OPTIONS, SettingsTarget, SettingsValues};

/// How Deck mode uses the screen.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DeckDisplay {
    /// Fill the monitor while in Deck mode.
    pub fullscreen: bool,
    /// Which monitor to fill (zero-based, left to right), or `None` for the one the window is on.
    pub monitor: Option<usize>,
}

impl DeckDisplay {
    /// From the Display rows of Settings. The monitor row's first choice is "Same as window", then
    /// "Monitor 1", "Monitor 2" and so on.
    pub fn from_settings(values: &SettingsValues) -> Self {
        let fullscreen = values.toggle(SettingsTarget::Global, KEY_DECK_FULLSCREEN, true);
        let monitor = values.choice(SettingsTarget::Global, KEY_DECK_DISPLAY, 0).checked_sub(1);
        Self { fullscreen, monitor }
    }
}

/// What the window should do now that the interface is `mode`.
///
/// `fullscreened` is whether Reclaw made it fullscreen itself, so that leaving Deck mode (or turning
/// the option off) gives the window back, while a window the user made fullscreen by hand is left
/// alone. `monitors` is how many are connected: a saved "Monitor 3" with two connected means the
/// monitor the window is on.
pub fn on_mode(mode: UiMode, display: DeckDisplay, fullscreened: bool, monitors: usize) -> Option<WindowCommand> {
    match (mode, display.fullscreen, fullscreened) {
        (UiMode::Deck, true, _) => Some(WindowCommand::Fullscreen { monitor: display.monitor.filter(|i| *i < monitors) }),
        (UiMode::Desktop, _, true) | (UiMode::Deck, false, true) => Some(WindowCommand::Windowed),
        _ => None,
    }
}

/// The scale the UI scale row asks for, on top of the system's. Untouched is 1.
pub fn ui_scale(values: &SettingsValues) -> f64 {
    match values.choice(SettingsTarget::Global, KEY_UI_SCALE, 0) {
        i if i < SCALE_OPTIONS.len() => [1.0, 1.25, 1.5][i],
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingValue;

    fn values(fullscreen: Option<bool>, monitor: Option<usize>, scale: Option<usize>) -> SettingsValues {
        let mut v = SettingsValues::default();
        if let Some(on) = fullscreen {
            v.set(SettingsTarget::Global, KEY_DECK_FULLSCREEN, SettingValue::Bool(on));
        }
        if let Some(i) = monitor {
            v.set(SettingsTarget::Global, KEY_DECK_DISPLAY, SettingValue::Choice(i));
        }
        if let Some(i) = scale {
            v.set(SettingsTarget::Global, KEY_UI_SCALE, SettingValue::Choice(i));
        }
        v
    }

    #[test]
    fn untouched_settings_fill_the_monitor_the_window_is_on() {
        let d = DeckDisplay::from_settings(&SettingsValues::default());
        assert_eq!(d, DeckDisplay { fullscreen: true, monitor: None });
    }

    #[test]
    fn the_monitor_row_counts_from_one_after_same_as_window() {
        assert_eq!(DeckDisplay::from_settings(&values(None, Some(0), None)).monitor, None);
        assert_eq!(DeckDisplay::from_settings(&values(None, Some(1), None)).monitor, Some(0));
        assert_eq!(DeckDisplay::from_settings(&values(None, Some(3), None)).monitor, Some(2));
        assert!(!DeckDisplay::from_settings(&values(Some(false), None, None)).fullscreen);
    }

    #[test]
    fn entering_deck_mode_fills_the_chosen_monitor() {
        let d = DeckDisplay { fullscreen: true, monitor: Some(1) };
        assert_eq!(on_mode(UiMode::Deck, d, false, 2), Some(WindowCommand::Fullscreen { monitor: Some(1) }));
    }

    #[test]
    fn a_monitor_that_is_not_there_means_the_one_the_window_is_on() {
        let d = DeckDisplay { fullscreen: true, monitor: Some(2) };
        assert_eq!(on_mode(UiMode::Deck, d, false, 2), Some(WindowCommand::Fullscreen { monitor: None }));
        assert_eq!(on_mode(UiMode::Deck, d, false, 0), Some(WindowCommand::Fullscreen { monitor: None }));
    }

    #[test]
    fn leaving_deck_mode_gives_the_window_back_only_if_we_took_it() {
        let d = DeckDisplay { fullscreen: true, monitor: None };
        assert_eq!(on_mode(UiMode::Desktop, d, true, 1), Some(WindowCommand::Windowed));
        assert_eq!(on_mode(UiMode::Desktop, d, false, 1), None, "the user's own fullscreen is theirs");
    }

    #[test]
    fn turning_the_option_off_in_deck_mode_gives_the_window_back() {
        let d = DeckDisplay { fullscreen: false, monitor: None };
        assert_eq!(on_mode(UiMode::Deck, d, true, 1), Some(WindowCommand::Windowed));
        assert_eq!(on_mode(UiMode::Deck, d, false, 1), None);
    }

    #[test]
    fn ui_scale_follows_the_row() {
        assert_eq!(ui_scale(&values(None, None, None)), 1.0);
        assert_eq!(ui_scale(&values(None, None, Some(1))), 1.25);
        assert_eq!(ui_scale(&values(None, None, Some(2))), 1.5);
        assert_eq!(ui_scale(&values(None, None, Some(9))), 1.0, "a value from a newer version is ignored");
    }
}
