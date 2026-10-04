//! Which interface is showing and whether a simulated keyboard is up. Pure, so the mode rules are
//! tested without a window.
use reclaw_input::UiMode;

use crate::effect::{Effect, ModePref};

/// The simulated on-screen keyboard covers this share of the window height, about what a phone
/// keyboard takes in landscape.
pub const SIM_KEYBOARD_SHARE: f32 = 0.45;

/// The mode a preference asks for. `Auto` follows what the environment detected at startup.
pub fn resolve(pref: ModePref, detected: UiMode) -> UiMode {
    match pref {
        ModePref::Auto => detected,
        ModePref::Desktop => UiMode::Desktop,
        ModePref::Deck => UiMode::Deck,
    }
}

pub fn other(mode: UiMode) -> UiMode {
    match mode {
        UiMode::Desktop => UiMode::Deck,
        UiMode::Deck => UiMode::Desktop,
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ShellModel {
    /// What the environment suggested at startup; `Auto` means this.
    pub detected: UiMode,
    pub pref: ModePref,
    pub mode: UiMode,
    /// Height of the simulated on-screen keyboard, 0 when hidden.
    pub keyboard: f32,
    /// Show the simulated keyboard whenever a text field takes focus, like a touch device would.
    pub keyboard_follows_focus: bool,
}

impl ShellModel {
    pub fn new(detected: UiMode, keyboard_follows_focus: bool) -> Self {
        Self { detected, pref: ModePref::Auto, mode: detected, keyboard: 0., keyboard_follows_focus }
    }

    /// Switch interface by hand (the F10 key, the Deck mode button). The choice sticks as an
    /// explicit preference rather than flipping back to what was detected.
    pub fn toggle_mode(&mut self) {
        self.set_pref(match other(self.mode) {
            UiMode::Desktop => ModePref::Desktop,
            UiMode::Deck => ModePref::Deck,
        });
    }

    pub fn set_pref(&mut self, pref: ModePref) {
        self.pref = pref;
        self.mode = resolve(pref, self.detected);
        self.keyboard = 0.;
    }

    /// Show or hide the simulated keyboard by hand (the F9 key).
    pub fn toggle_keyboard(&mut self, window_height: f32) {
        self.keyboard = if self.keyboard > 0. { 0. } else { keyboard_height(window_height) };
    }

    /// React to a command from the UI that is the shell's to handle. Others pass to the host.
    pub fn on_effect(&mut self, effect: &Effect, window_height: f32) {
        match effect {
            Effect::SwitchToDesktop => self.set_pref(ModePref::Desktop),
            Effect::SetMode(pref) => self.set_pref(*pref),
            Effect::BeginTextEntry(_) if self.keyboard_follows_focus => self.keyboard = keyboard_height(window_height),
            Effect::EndTextEntry(_) if self.keyboard_follows_focus => self.keyboard = 0.,
            _ => {}
        }
    }
}

pub fn keyboard_height(window_height: f32) -> f32 {
    (window_height * SIM_KEYBOARD_SHARE).round()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deck::settings::TextField;

    #[test]
    fn auto_follows_detection_and_explicit_choices_win() {
        assert_eq!(resolve(ModePref::Auto, UiMode::Deck), UiMode::Deck);
        assert_eq!(resolve(ModePref::Auto, UiMode::Desktop), UiMode::Desktop);
        assert_eq!(resolve(ModePref::Desktop, UiMode::Deck), UiMode::Desktop);
        assert_eq!(resolve(ModePref::Deck, UiMode::Desktop), UiMode::Deck);
    }

    #[test]
    fn toggling_twice_returns_and_the_choice_sticks() {
        let mut m = ShellModel::new(UiMode::Desktop, false);
        m.toggle_mode();
        assert_eq!((m.mode, m.pref), (UiMode::Deck, ModePref::Deck));
        m.toggle_mode();
        assert_eq!((m.mode, m.pref), (UiMode::Desktop, ModePref::Desktop));
    }

    #[test]
    fn the_deck_menu_and_settings_switch_through_effects() {
        let mut m = ShellModel::new(UiMode::Deck, false);
        m.on_effect(&Effect::SwitchToDesktop, 800.);
        assert_eq!(m.mode, UiMode::Desktop);
        m.on_effect(&Effect::SetMode(ModePref::Auto), 800.);
        assert_eq!(m.mode, UiMode::Deck, "Auto goes back to what was detected");
    }

    #[test]
    fn the_simulated_keyboard_is_a_share_of_the_window_and_leaves_with_the_mode() {
        let mut m = ShellModel::new(UiMode::Deck, false);
        m.toggle_keyboard(800.);
        assert_eq!(m.keyboard, 360.);
        m.toggle_mode();
        assert_eq!(m.keyboard, 0., "a mode switch drops the keyboard");
        m.toggle_keyboard(800.);
        m.toggle_keyboard(800.);
        assert_eq!(m.keyboard, 0.);
    }

    #[test]
    fn text_entry_raises_the_keyboard_only_when_asked_to() {
        let field = TextField::InstallLocation;
        let mut off = ShellModel::new(UiMode::Deck, false);
        off.on_effect(&Effect::BeginTextEntry(field), 800.);
        assert_eq!(off.keyboard, 0.);

        let mut on = ShellModel::new(UiMode::Deck, true);
        on.on_effect(&Effect::BeginTextEntry(field), 800.);
        assert_eq!(on.keyboard, 360.);
        on.on_effect(&Effect::EndTextEntry(field), 800.);
        assert_eq!(on.keyboard, 0.);
    }
}
