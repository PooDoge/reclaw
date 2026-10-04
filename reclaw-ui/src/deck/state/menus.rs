//! The cascading menus: an app's Options menu and the pickers behind choice rows.
use super::{DeckState, DeckView, Effect, Overlay, Screen, ids, types::*};
use crate::{
    app_menu::options_menu,
    deck::settings::{Row, RowKind, SettingValue, SettingsTarget},
    surface::{MenuEntry, MenuState, Outcome},
};
use reclaw_input::{Action, Direction};

/// A picker for a choice row. No Cancel row: Back closes it, and a tap outside dismisses it.
pub fn choice_menu(row: &Row, current: usize) -> MenuState<MenuAction> {
    let RowKind::Choice { options, .. } = &row.kind else { return MenuState::new(row.label, Vec::new()) };
    let mut menu =
        MenuState::new(row.label, options.iter().enumerate().map(|(i, o)| MenuEntry::action(*o, MenuAction::Choice(i))).collect());
    for _ in 0..current {
        menu.navigate(Direction::Down);
    }
    menu
}

impl DeckState {
    pub(super) fn open_options(&mut self, view: &DeckView) {
        let app = match (self.screen, ids::tile_game(self.focus)) {
            (Screen::Game(id), _) | (Screen::Settings(SettingsTarget::App(id)), _) => Some(id),
            (Screen::Home, Some(id)) => Some(id),
            _ => None,
        };
        if let (Overlay::None, Some(game)) = (self.overlay, app.and_then(|id| view.game(id))) {
            self.leave_scope();
            self.menu = Some(options_menu(game, true));
            self.overlay = Overlay::Menu(MenuPurpose::Options(game.id));
        }
    }

    pub(super) fn open_choice(&mut self, target: SettingsTarget, row: &Row, _view: &DeckView) {
        let RowKind::Choice { default, .. } = &row.kind else { return };
        let current = self.values.choice(target, row.key, *default);
        self.leave_scope();
        self.menu = Some(choice_menu(row, current));
        self.overlay = Overlay::Menu(MenuPurpose::Choice(target, row.key));
    }

    /// The picker of a launch row: "Default" or "Game's own" first, then every value on offer, the
    /// current one under the cursor.
    pub(super) fn open_launch_choice(
        &mut self,
        target: SettingsTarget,
        row: &Row,
        key: reclaw_games::settings::SettingKey,
        view: &DeckView,
    ) {
        let Some(control) = view.launch.and_then(|l| l.control(target, key)) else { return };
        let entries = control.options.iter().enumerate().map(|(i, o)| MenuEntry::action(o.label.clone(), MenuAction::Choice(i))).collect();
        let mut menu = MenuState::new(row.label, entries);
        for _ in 0..control.selected {
            menu.navigate(Direction::Down);
        }
        self.leave_scope();
        self.menu = Some(menu);
        self.overlay = Overlay::Menu(MenuPurpose::Launch(target, key));
    }

    pub(super) fn close_menu(&mut self, view: &DeckView) {
        self.menu = None;
        self.overlay = Overlay::None;
        self.enter_scope(view);
    }

    pub(super) fn menu_action(&mut self, action: Action, view: &DeckView, fx: &mut Vec<Effect>) {
        let Overlay::Menu(purpose) = self.overlay else { return };
        let Some(menu) = self.menu.as_mut() else { return };
        let outcome = match action {
            Action::Navigate(dir) => menu.navigate(dir),
            Action::Confirm => menu.confirm(),
            Action::Back => menu.back(),
            _ => Outcome::None,
        };
        self.after_menu(purpose, outcome, view, fx);
    }

    /// Pointer or touch press on a menu row.
    pub fn pick_menu(&mut self, level: usize, index: usize, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.last_input = LastInput::Pointer;
        if let (Overlay::Menu(purpose), Some(menu)) = (self.overlay, self.menu.as_mut()) {
            let outcome = menu.pick(level, index);
            self.after_menu(purpose, outcome, view, &mut fx);
        }
        fx
    }

    fn after_menu(&mut self, purpose: MenuPurpose, outcome: Outcome<MenuAction>, view: &DeckView, fx: &mut Vec<Effect>) {
        match outcome {
            Outcome::Closed => self.close_menu(view),
            Outcome::Chose(choice) => self.on_menu_choice(purpose, choice, view, fx),
            Outcome::Moved | Outcome::None => {}
        }
    }

    fn on_menu_choice(&mut self, purpose: MenuPurpose, choice: MenuAction, view: &DeckView, fx: &mut Vec<Effect>) {
        match (purpose, choice) {
            (MenuPurpose::Choice(target, key), MenuAction::Choice(i)) => {
                self.close_menu(view);
                self.set_value(target, key, SettingValue::Choice(i), fx);
            }
            (MenuPurpose::Launch(target, key), MenuAction::Choice(i)) => {
                self.close_menu(view);
                // The options are worked out again here, so a pick means what was on screen.
                let value = view.launch.and_then(|l| l.control(target, key)).and_then(|c| c.options.into_iter().nth(i)).map(|o| o.value);
                if let Some(value) = value {
                    fx.push(Effect::LaunchSetting { app: target.app(), key, value });
                }
            }
            (MenuPurpose::Options(app), MenuAction::Uninstall) => {
                self.close_menu(view);
                self.open_confirm(ConfirmKind::Uninstall(app), view);
            }
            (MenuPurpose::Options(app), MenuAction::Properties) => {
                self.close_menu(view);
                self.open_settings(SettingsTarget::App(app), view);
            }
            (MenuPurpose::Options(app), choice) => {
                self.close_menu(view);
                fx.extend(choice.effect(app));
            }
            _ => self.close_menu(view),
        }
    }

    pub(super) fn open_confirm(&mut self, kind: ConfirmKind, _view: &DeckView) {
        self.leave_scope();
        self.overlay = Overlay::Confirm(kind);
        self.focus = super::ids::CONFIRM_CANCEL;
    }
}
