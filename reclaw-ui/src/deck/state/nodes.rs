//! The declared focus layout: for each scope, the rectangles a gamepad can land on. Rectangles are
//! computed from layout constants, not measured, so they are testable and match the renderer's
//! scroll math.
use reclaw_input::{FocusId, FocusNode, Rect};

use super::{
    DeckState, DeckView, Overlay, Screen, Section, ids,
    scope::{Scope, SettingsPane},
    types::*,
    view::{BANNER_H, shelves_of, tile_rect},
};
use crate::{deck::shows_stop_pair, metrics::*};

pub(super) fn node(id: FocusId, x: f32, y: f32, w: f32, h: f32) -> FocusNode {
    FocusNode { id, rect: Rect::new(x, y, w, h) }
}

impl DeckState {
    pub(super) fn pane(&self) -> SettingsPane {
        if self.two_pane() {
            SettingsPane::TwoPane(self.settings_section)
        } else if self.drilled {
            SettingsPane::Detail(self.settings_section)
        } else {
            SettingsPane::List
        }
    }

    pub(super) fn scope(&self) -> Scope {
        match self.overlay {
            Overlay::MainMenu => Scope::MainMenu,
            Overlay::QuickAccess => Scope::QuickAccess,
            Overlay::Menu(_) => Scope::Menu,
            Overlay::Confirm(kind) => Scope::Confirm(kind),
            Overlay::Notice(id) => Scope::Notice(id),
            Overlay::None => match self.screen {
                Screen::Home => Scope::Home(self.section),
                Screen::Game(id) => Scope::Game(id),
                Screen::Install(id) => Scope::Install(id),
                Screen::Settings(target) => Scope::Settings(target, self.pane()),
            },
        }
    }

    pub fn nodes(&self, view: &DeckView) -> Vec<FocusNode> {
        self.nodes_for(self.scope(), view)
    }

    pub(super) fn nodes_for(&self, scope: Scope, view: &DeckView) -> Vec<FocusNode> {
        match scope {
            Scope::MainMenu => {
                (0..MAIN_MENU.len()).map(|i| node(ids::menu(i), 0., i as f32 * DECK_ROW_H, DECK_PANEL_W, DECK_ROW_H)).collect()
            }
            Scope::QuickAccess => {
                let mut v = Vec::new();
                if view.active_game().is_some() {
                    v.push(node(ids::QA_RESUME, 0., 0., 190., DECK_TARGET_MIN));
                    v.push(node(ids::QA_STOP, 206., 0., 150., DECK_TARGET_MIN));
                }
                v.push(node(ids::QA_DOWNLOADS, 0., 200., DECK_PANEL_W, DECK_ROW_H));
                for i in 0..view.recents.len().min(super::view::QA_RECENTS) {
                    v.push(node(ids::qa_recent(i), 0., 300. + i as f32 * DECK_ROW_H, DECK_PANEL_W, DECK_ROW_H));
                }
                v
            }
            Scope::Menu => Vec::new(),
            Scope::Confirm(_) => {
                vec![node(ids::CONFIRM_CANCEL, 0., 0., 200., DECK_TARGET_MIN), node(ids::CONFIRM_OK, 216., 0., 200., DECK_TARGET_MIN)]
            }
            Scope::Notice(_) => {
                vec![node(ids::NOTICE_CLOSE, 0., 0., 200., DECK_TARGET_MIN), node(ids::NOTICE_DISMISS, 216., 0., 200., DECK_TARGET_MIN)]
            }
            Scope::Game(id) => {
                let mut v = vec![node(ids::GAME_PRIMARY, 0., 0., 220., DECK_TARGET_MIN)];
                if view.game(id).is_some_and(|g| shows_stop_pair(&g.run, true)) {
                    v.push(node(ids::GAME_STOP, 236., 0., 150., DECK_TARGET_MIN));
                }
                v.push(node(ids::GAME_FOLDER, 0., 80., 220., DECK_TARGET_MIN));
                v.push(node(ids::GAME_MANAGE, 236., 80., 150., DECK_TARGET_MIN));
                v
            }
            Scope::Install(_) => self.install_nodes(),
            Scope::Settings(target, pane) => self.settings_nodes(target, pane, view),
            Scope::Home(section @ (Section::Library | Section::Catalog)) => {
                // The Now Playing banner tops the Library only.
                let banner = section == Section::Library && view.active_game().is_some();
                let mut v = Vec::new();
                if banner {
                    v.push(node(ids::BANNER_RESUME, 0., 0., 190., BANNER_H));
                    v.push(node(ids::BANNER_STOP, 206., 0., 150., BANNER_H));
                }
                for (s, shelf) in shelves_of(view, section).iter().enumerate() {
                    for (t, game) in shelf.games.iter().enumerate() {
                        v.push(FocusNode { id: ids::tile(s, *game), rect: tile_rect(s, t, banner) });
                    }
                }
                v
            }
            Scope::Home(Section::Downloads) => view
                .downloads
                .iter()
                .enumerate()
                .map(|(i, d)| node(ids::download_cancel(d.id), 0., i as f32 * 96., DECK_TARGET_MIN, DECK_TARGET_MIN))
                .collect(),
            Scope::Home(_) => Vec::new(),
        }
    }

    /// The part of the focused target a scrolling page must keep on screen: its top and bottom in
    /// the page body's coordinates. `None` for targets outside the scrolling body (footer buttons,
    /// the section list of a two-pane settings page, overlays).
    pub fn reveal_target(&self, view: &DeckView) -> Option<(f32, f32)> {
        let in_body = match self.scope() {
            Scope::Install(_) => ![ids::INSTALL_CANCEL, ids::INSTALL_SUBMIT].contains(&self.focus),
            Scope::Settings(..) => ids::row_of(self.focus).is_some(),
            _ => false,
        };
        if !in_body {
            return None;
        }
        let rect = self.nodes(view).into_iter().find(|n| n.id == self.focus)?.rect;
        Some((rect.y, rect.y + rect.h))
    }

    pub(super) fn default_focus(&self, scope: Scope, view: &DeckView) -> FocusId {
        let nodes = self.nodes_for(scope, view);
        let first = nodes.first().map(|n| n.id).unwrap_or(FocusId(0));
        match scope {
            Scope::MainMenu => ids::menu(MAIN_MENU.iter().position(|m| *m == MainMenuEntry::Section(self.section)).unwrap_or(0)),
            // Destructive confirmations start on Cancel.
            Scope::Confirm(_) => ids::CONFIRM_CANCEL,
            // Closing is the harmless choice; it is where focus starts.
            Scope::Notice(_) => ids::NOTICE_CLOSE,
            Scope::Settings(_, SettingsPane::TwoPane(s)) => ids::settings_nav(s),
            Scope::Settings(_, SettingsPane::List) => ids::settings_nav(self.settings_section),
            _ => first,
        }
    }

    pub(super) fn remembered(&self, scope: Scope, view: &DeckView) -> FocusId {
        let nodes = self.nodes_for(scope, view);
        match self.memory.get(&scope) {
            Some(id) if nodes.iter().any(|n| n.id == *id) => *id,
            _ => self.default_focus(scope, view),
        }
    }

    pub(super) fn enter_scope(&mut self, view: &DeckView) {
        self.focus = self.remembered(self.scope(), view);
    }

    pub(super) fn leave_scope(&mut self) {
        self.memory.insert(self.scope(), self.focus);
    }
}
