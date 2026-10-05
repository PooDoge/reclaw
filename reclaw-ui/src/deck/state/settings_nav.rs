//! The Settings and Properties pages: two panes (sections beside rows) on wide windows, a
//! drill-down (list, then rows) on narrow ones.
use reclaw_input::{FocusId, FocusNode};

use super::{DeckState, DeckView, Effect, Screen, ids, nodes::node, scope::SettingsPane, types::*};
use crate::{
    deck::settings::{
        CredentialPart, GlobalAction, RowAction, RowKind, Schema, SettingValue, SettingsTarget, TextField, app_properties, global_settings,
        section_slots,
    },
    metrics::*,
};

/// Left of the rows in a two-pane layout.
const ROWS_X: f32 = DECK_SETTINGS_NAV_W + 48.;

impl DeckState {
    pub fn settings_schema(&self, target: SettingsTarget, view: &DeckView) -> Option<Schema> {
        // Launch rows: only what the display and (for a game) the game agree on.
        let specs = view.launch.map(|launch| launch.specs(target)).unwrap_or_default();
        let unknown = reclaw_games::settings::DisplayEnvironment::unknown();
        let displays = view.launch.map_or(&unknown, |launch| launch.env);
        match target {
            SettingsTarget::Global => Some(global_settings(&specs, displays)),
            SettingsTarget::App(id) => view.game(id).map(|game| app_properties(game, &specs)),
        }
    }

    pub(super) fn settings_nodes(&self, target: SettingsTarget, pane: SettingsPane, view: &DeckView) -> Vec<FocusNode> {
        let Some(schema) = self.settings_schema(target, view) else { return Vec::new() };
        let nav = |width: f32| {
            (0..schema.sections.len()).map(|i| node(ids::settings_nav(i), 0., i as f32 * DECK_ROW_H, width, DECK_ROW_H)).collect::<Vec<_>>()
        };
        let rows = |section: usize, x: f32, width: f32| {
            let Some(s) = schema.sections.get(section) else { return Vec::new() };
            section_slots(s, Density::Controller)
                .0
                .into_iter()
                .map(|slot| node(ids::settings_row(section, slot.row), x, slot.y, width, slot.h))
                .collect::<Vec<_>>()
        };
        match pane {
            SettingsPane::TwoPane(s) => [nav(DECK_SETTINGS_NAV_W), rows(s, ROWS_X, SURFACE_MAX_W)].concat(),
            SettingsPane::List => nav(SURFACE_MAX_W),
            SettingsPane::Detail(s) => rows(s, 0., SURFACE_MAX_W),
        }
    }

    pub(super) fn open_settings(&mut self, target: SettingsTarget, view: &DeckView) {
        self.settings_section = 0;
        self.drilled = false;
        self.go(Screen::Settings(target), view);
    }

    /// Back inside settings: Detail to List first, then leave.
    pub(super) fn settings_back(&mut self, view: &DeckView) {
        if self.drilled && !self.two_pane() {
            self.leave_scope();
            self.drilled = false;
            self.enter_scope(view);
        } else {
            self.drilled = false;
            self.go_back(view);
        }
    }

    /// Left from the rows returns to the section being shown, not to whichever section happens to
    /// sit level with the row. Returns whether it handled the move.
    pub(super) fn settings_nav_left(&mut self, dir: reclaw_input::Direction) -> bool {
        let in_rows = matches!(self.screen, Screen::Settings(_)) && self.two_pane() && ids::row_of(self.focus).is_some();
        if in_rows && dir == reclaw_input::Direction::Left {
            self.focus = ids::settings_nav(self.settings_section);
            return true;
        }
        false
    }

    /// In two-pane mode the rows follow the section list as focus moves down it.
    pub(super) fn follow_nav_focus(&mut self) {
        if let (Screen::Settings(_), true, Some(section)) = (self.screen, self.two_pane(), ids::nav_section(self.focus)) {
            self.settings_section = section;
        }
    }

    pub(super) fn activate_settings(&mut self, target: SettingsTarget, id: FocusId, view: &DeckView, fx: &mut Vec<Effect>) {
        if let Some(section) = ids::nav_section(id) {
            self.settings_section = section;
            if self.two_pane() {
                // Entering the rows from the section list.
                if let Some(first) = self.nodes(view).into_iter().find(|n| ids::row_of(n.id).is_some()) {
                    self.focus = first.id;
                }
            } else {
                self.leave_scope();
                self.drilled = true;
                self.enter_scope(view);
            }
            return;
        }
        let (Some((section, row)), Some(schema)) = (ids::row_of(id), self.settings_schema(target, view)) else { return };
        let Some(row) = schema.row(section, row) else { return };
        match &row.kind {
            RowKind::Toggle { default } => {
                let value = !self.values.toggle(target, row.key, *default);
                self.set_value(target, row.key, SettingValue::Bool(value), fx);
            }
            RowKind::Choice { .. } => self.open_choice(target, row, view),
            RowKind::Text { field, .. } => self.begin_entry(*field, fx),
            RowKind::Action { action, .. } => self.run_row_action(target, *action, view, fx),
            RowKind::Launch { key } => self.open_launch_choice(target, row, *key, view),
            RowKind::Credential { provider, part } => match part {
                CredentialPart::Status => {}
                CredentialPart::Token => self.begin_entry(TextField::for_provider(*provider), fx),
                CredentialPart::Save => fx.push(Effect::SubmitToken(*provider)),
                CredentialPart::Check => fx.push(Effect::CheckToken(*provider)),
                CredentialPart::Create => fx.push(Effect::OpenUrl(provider.token_page().to_string())),
                CredentialPart::Remove => fx.push(Effect::RemoveToken(*provider)),
            },
            RowKind::Global { action } => fx.push(match action {
                GlobalAction::OpenLogFolder => Effect::OpenLogFolder,
                GlobalAction::SaveDiagnostics => Effect::SaveDiagnostics,
                GlobalAction::UpdateSources => Effect::UpdateSources,
            }),
            RowKind::Info { .. } => {}
        }
    }

    pub(super) fn set_value(&mut self, target: SettingsTarget, key: &'static str, value: SettingValue, fx: &mut Vec<Effect>) {
        self.values.set(target, key, value);
        fx.extend(Effect::setting(target, key, value));
    }

    fn run_row_action(&mut self, target: SettingsTarget, action: RowAction, view: &DeckView, fx: &mut Vec<Effect>) {
        let Some(app) = target.app() else { return };
        match action {
            RowAction::OpenFolder => fx.push(Effect::OpenFolder(app)),
            RowAction::Verify => fx.push(Effect::Verify(app)),
            RowAction::CheckUpdate => fx.push(Effect::CheckUpdate(app)),
            RowAction::Uninstall => self.open_confirm(ConfirmKind::Uninstall(app), view),
        }
    }
}
