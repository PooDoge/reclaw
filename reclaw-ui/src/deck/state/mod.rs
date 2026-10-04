//! Deck mode's brain: which page, which overlay, what is focused, who owns the pad, and what the
//! host should do. Pure data in, pure data out: no Freya types, so it is tested like any logic.
//!
//! Layout of this module (each file is one responsibility):
//! * `types` / `ids` / `view`: the data vocabulary;
//! * `scope` / `nodes`: the declared focus layout of every page and overlay;
//! * `reducer`: actions in, effects out; `activate`: what pressing a target does;
//! * `menus`: the Options menu and pickers; `settings_nav` / `install`: the form-like pages.
use std::collections::HashMap;

use reclaw_input::{ControllerKind, FocusId, InputOwner};

use super::settings::SettingsValues;
use crate::surface::MenuState;

mod activate;
pub mod ids;
mod install;
mod menus;
mod nodes;
mod reducer;
mod route;
mod scope;
mod screens;
mod settings_nav;
mod types;
mod view;

#[cfg(test)]
mod tests;

pub use install::InstallDraft;
pub use types::*;
pub use view::{BANNER_BLOCK, BANNER_H, DeckView, QA_RECENTS, SHELF_H, SHELF_TITLE_BLOCK, ShelfSpec, shelf_top, shelves, tile_rect};

use scope::Scope;

/// Below this window width the settings page drills down (list, then rows) instead of showing both.
pub const TWO_PANE_MIN_W: f32 = 900.;

#[derive(Clone, Debug)]
pub struct DeckState {
    section: Section,
    screen: Screen,
    /// Screens to return to, innermost last.
    stack: Vec<Screen>,
    overlay: Overlay,
    focus: FocusId,
    memory: HashMap<Scope, FocusId>,
    in_front: bool,
    /// The overlay was opened over a running app (via Guide): closing it should send us back.
    came_from_game: bool,
    last_input: LastInput,
    owner: InputOwner,
    window: (f32, f32),
    /// The text box being typed into, if any.
    entry: Option<TextField>,
    menu: Option<MenuState<MenuAction>>,
    settings_section: usize,
    /// Narrow windows: showing a section's rows rather than the list of sections.
    drilled: bool,
    values: SettingsValues,
    install: InstallDraft,
    /// Requests for the router made by the last action, handed out with its effects.
    nav_requests: Vec<Effect>,
    /// The button being held for a notification's hold gesture, while its ring is drawn.
    holding: Option<reclaw_input::Button>,
    /// Whether the pad reader has been asked to treat X and Y as holds.
    holds_on: bool,
}

impl DeckState {
    pub fn new(view: &DeckView) -> Self {
        let mut s = Self {
            section: Section::Library,
            screen: Screen::Home,
            stack: Vec::new(),
            overlay: Overlay::None,
            focus: FocusId(0),
            memory: HashMap::new(),
            in_front: true,
            came_from_game: false,
            last_input: LastInput::Gamepad(ControllerKind::Generic),
            owner: InputOwner::Launcher,
            window: (1280., 800.),
            entry: None,
            menu: None,
            settings_section: 0,
            drilled: false,
            values: SettingsValues::default(),
            install: InstallDraft::default(),
            nav_requests: Vec::new(),
            holding: None,
            holds_on: false,
        };
        s.focus = s.default_focus(s.scope(), view);
        s.owner = s.input_owner(view);
        s
    }

    pub fn section(&self) -> Section {
        self.section
    }

    pub fn screen(&self) -> Screen {
        self.screen
    }

    pub fn overlay(&self) -> Overlay {
        self.overlay
    }

    pub fn focus(&self) -> FocusId {
        self.focus
    }

    pub fn last_input(&self) -> LastInput {
        self.last_input
    }

    pub fn set_last_input(&mut self, last: LastInput) {
        self.last_input = last;
    }

    /// The button whose hold ring is being drawn.
    pub fn holding(&self) -> Option<reclaw_input::Button> {
        self.holding
    }

    /// Whether the notification toast is up: there is one, and nothing else holds the screen.
    pub fn toast_visible(&self, view: &DeckView) -> bool {
        view.top_notice().is_some() && self.overlay == Overlay::None && self.entry.is_none()
    }

    pub fn in_front(&self) -> bool {
        self.in_front
    }

    pub fn window(&self) -> (f32, f32) {
        self.window
    }

    /// The window size decides how the settings page lays out; focus is repaired if the layout
    /// change took its target away.
    pub fn set_window(&mut self, window: (f32, f32), view: &DeckView) {
        self.window = window;
        if !self.nodes(view).iter().any(|n| n.id == self.focus) && self.menu.is_none() {
            self.focus = self.remembered(self.scope(), view);
        }
    }

    /// The ring is drawn only while the gamepad or keyboard is driving.
    pub fn focus_visible(&self) -> bool {
        !matches!(self.last_input, LastInput::Pointer)
    }

    /// The text box being typed into. While set, only Confirm and Back are handled.
    pub fn text_entry(&self) -> Option<TextField> {
        self.entry
    }

    /// The open menu, for rendering. `Some` exactly while `overlay()` is `Overlay::Menu`.
    pub fn menu(&self) -> Option<&MenuState<MenuAction>> {
        self.menu.as_ref()
    }

    pub fn settings_section(&self) -> usize {
        self.settings_section
    }

    pub fn drilled(&self) -> bool {
        self.drilled
    }

    pub fn two_pane(&self) -> bool {
        self.window.0 >= TWO_PANE_MIN_W
    }

    pub fn values(&self) -> &SettingsValues {
        &self.values
    }

    /// Seed persisted settings.
    /// Replace the working copy with the store's, which both interfaces share.
    pub fn sync_values(&mut self, values: &SettingsValues) {
        if self.values != *values {
            self.values = values.clone();
        }
    }

    pub fn values_mut(&mut self) -> &mut SettingsValues {
        &mut self.values
    }

    pub fn install_draft(&self) -> &InstallDraft {
        &self.install
    }

    pub fn settings_target(&self) -> Option<SettingsTarget> {
        match self.screen {
            Screen::Settings(t) => Some(t),
            _ => None,
        }
    }

    /// Who should get the gamepad right now.
    pub fn input_owner(&self, view: &DeckView) -> InputOwner {
        if view.active_game().is_some() && !self.in_front { InputOwner::App } else { InputOwner::Launcher }
    }
}
