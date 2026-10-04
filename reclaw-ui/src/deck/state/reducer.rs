//! Actions in, effects out. Everything that changes `DeckState` in response to input starts here;
//! what a press on a particular target does is in `activate`.
use reclaw_input::{Action, Direction, FocusId, next_focus};

use super::{DeckState, DeckView, Effect, LastInput, Overlay, Screen, ids, types::TextField};

impl DeckState {
    pub fn apply(&mut self, action: Action, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.apply_inner(action, view, &mut fx);
        self.sync_owner(view, &mut fx);
        fx
    }

    /// Pointer click on a focus target: focus it and confirm.
    pub fn click(&mut self, id: FocusId, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.last_input = LastInput::Pointer;
        if self.entry.is_some() {
            self.end_entry(&mut fx);
        }
        if self.has_target(id, view) {
            if id != ids::PAGE_BACK {
                self.focus = id;
            }
            self.activate(id, view, &mut fx);
        }
        self.sync_owner(view, &mut fx);
        fx
    }

    /// A tap outside an overlay (on its scrim): close it entirely.
    pub fn dismiss(&mut self, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        self.last_input = LastInput::Pointer;
        match self.overlay {
            Overlay::None => {}
            Overlay::Menu(_) => self.close_menu(view),
            _ => self.close_overlay(view, &mut fx),
        }
        self.sync_owner(view, &mut fx);
        fx
    }

    /// Call when the host's data changed (a run state moved, a download finished). Repairs focus
    /// that points at something that no longer exists and reports an ownership change.
    pub fn sync(&mut self, view: &DeckView) -> Vec<Effect> {
        let mut fx = Vec::new();
        let missing = |s: Screen| match s {
            Screen::Game(id) | Screen::Install(id) => view.game(id).is_none(),
            Screen::Settings(t) => t.app().is_some_and(|id| view.game(id).is_none()),
            Screen::Home => false,
        };
        if missing(self.screen) {
            self.stack.clear();
            self.screen = Screen::Home;
            self.drilled = false;
        }
        if self.overlay != Overlay::None && self.overlay_target_missing(view) {
            self.menu = None;
            self.overlay = Overlay::None;
        }
        if !self.nodes(view).iter().any(|n| n.id == self.focus) && !matches!(self.overlay, Overlay::Menu(_)) {
            self.focus = self.remembered(self.scope(), view);
        }
        self.sync_owner(view, &mut fx);
        fx
    }

    fn overlay_target_missing(&self, view: &DeckView) -> bool {
        use super::{ConfirmKind, MenuPurpose};
        match self.overlay {
            Overlay::Menu(MenuPurpose::Options(id)) | Overlay::Confirm(ConfirmKind::Uninstall(id)) => view.game(id).is_none(),
            _ => false,
        }
    }

    /// The running app ended: the launcher is the foreground again and owns the pad.
    pub fn on_app_ended(&mut self, view: &DeckView) -> Vec<Effect> {
        self.in_front = true;
        self.came_from_game = false;
        let mut fx = vec![Effect::BringLauncherToFront];
        fx.extend(self.sync(view));
        fx
    }

    pub(super) fn sync_owner(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        let owner = self.input_owner(view);
        if owner != self.owner {
            self.owner = owner;
            fx.push(Effect::InputOwner(owner));
        }
    }

    fn apply_inner(&mut self, action: Action, view: &DeckView, fx: &mut Vec<Effect>) {
        // Typing: the text box has the keyboard; Confirm and Back leave it, nothing else counts.
        if self.entry.is_some() {
            if matches!(action, Action::Confirm | Action::Back) {
                self.end_entry(fx);
            }
            return;
        }
        if matches!(self.overlay, Overlay::Menu(_)) {
            return self.menu_action(action, view, fx);
        }
        let plain = self.overlay == Overlay::None;
        match action {
            Action::Navigate(dir) => self.navigate(dir, view),
            Action::PageUp => (0..4).for_each(|_| self.navigate(Direction::Left, view)),
            Action::PageDown => (0..4).for_each(|_| self.navigate(Direction::Right, view)),
            Action::Confirm => {
                let id = self.focus;
                self.activate(id, view, fx);
            }
            Action::Back => self.back(view, fx),
            Action::MainMenu if self.overlay_allows_panels() => self.toggle_overlay(Overlay::MainMenu, view, fx),
            Action::QuickAccess if self.overlay_allows_panels() => self.toggle_overlay(Overlay::QuickAccess, view, fx),
            Action::PrevSection | Action::NextSection if plain && self.screen == Screen::Home => {
                self.leave_scope();
                self.section = self.section.step(action == Action::NextSection);
                self.enter_scope(view);
            }
            Action::Secondary | Action::Options if plain => self.open_options(view),
            Action::Tertiary => fx.push(Effect::Search),
            _ => {}
        }
    }

    /// The side panels open from the main pages and from each other, never over a confirmation.
    fn overlay_allows_panels(&self) -> bool {
        !matches!(self.overlay, Overlay::Confirm(_) | Overlay::Menu(_)) && self.entry.is_none()
    }

    fn navigate(&mut self, dir: Direction, view: &DeckView) {
        if self.settings_nav_left(dir) {
            return;
        }
        if let Some(next) = next_focus(&self.nodes(view), self.focus, dir) {
            self.focus = next;
            self.follow_nav_focus();
        }
    }

    pub(super) fn back(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay != Overlay::None {
            return self.close_overlay(view, fx);
        }
        match self.screen {
            Screen::Home => {}
            Screen::Settings(_) => self.settings_back(view),
            Screen::Game(_) | Screen::Install(_) => self.go_back(view),
        }
    }

    fn toggle_overlay(&mut self, which: Overlay, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay == which {
            return self.close_overlay(view, fx);
        }
        if self.overlay != Overlay::None {
            // Switching straight from one panel to the other.
            self.leave_scope();
            self.overlay = which;
            self.enter_scope(view);
            return;
        }
        if view.active_game().is_some() && !self.in_front {
            self.in_front = true;
            self.came_from_game = true;
            fx.push(Effect::BringLauncherToFront);
        }
        self.leave_scope();
        self.overlay = which;
        self.enter_scope(view);
    }

    pub(super) fn close_overlay(&mut self, view: &DeckView, fx: &mut Vec<Effect>) {
        self.close_overlay_quietly(view);
        if self.came_from_game && view.active_game().is_some() {
            self.came_from_game = false;
            self.in_front = false;
            fx.push(Effect::SendLauncherToBack);
        }
    }

    pub(super) fn begin_entry(&mut self, field: TextField, fx: &mut Vec<Effect>) {
        self.entry = Some(field);
        fx.push(Effect::BeginTextEntry(field));
    }

    pub(super) fn end_entry(&mut self, fx: &mut Vec<Effect>) {
        if let Some(field) = self.entry.take() {
            fx.push(Effect::EndTextEntry(field));
        }
    }

    /// Whether `id` is a target of the page that is showing (used by hosts to ignore stale clicks).
    pub fn has_target(&self, id: FocusId, view: &DeckView) -> bool {
        id == ids::PAGE_BACK || self.nodes(view).iter().any(|n| n.id == id)
    }
}
