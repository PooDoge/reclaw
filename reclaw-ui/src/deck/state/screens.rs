//! Moving between pages, and the focus bookkeeping that goes with it.
use super::{DeckState, DeckView, Effect, Overlay, Screen, Section};

impl DeckState {
    /// Open `screen`, remembering where we came from and where focus was.
    pub(super) fn go(&mut self, screen: Screen, view: &DeckView) {
        self.leave_scope();
        self.stack.push(self.screen);
        self.screen = screen;
        self.enter_scope(view);
        self.request_route();
    }

    /// Back to the previous page; Home when there is none.
    pub(super) fn go_back(&mut self, view: &DeckView) {
        self.leave_scope();
        self.screen = self.stack.pop().unwrap_or(Screen::Home);
        self.enter_scope(view);
        self.request_back();
    }

    /// Browse to a section's home page, forgetting the trail.
    pub(super) fn go_to_section(&mut self, section: Section, view: &DeckView) {
        // Leaving for another page means the user is browsing, not returning to the game.
        self.came_from_game = false;
        self.leave_scope();
        self.overlay = Overlay::None;
        self.menu = None;
        self.stack.clear();
        self.screen = Screen::Home;
        self.section = section;
        self.enter_scope(view);
        self.request_route();
    }

    /// Close any overlay without the return-to-the-game bookkeeping, so a page can be opened next.
    pub(super) fn close_overlay_quietly(&mut self, view: &DeckView) {
        self.leave_scope();
        self.overlay = Overlay::None;
        self.menu = None;
        self.enter_scope(view);
    }

    pub(super) fn resume(&mut self, app: u32, view: &DeckView, fx: &mut Vec<Effect>) {
        if self.overlay != Overlay::None {
            self.close_overlay_quietly(view);
        }
        self.came_from_game = false;
        self.in_front = false;
        fx.push(Effect::Resume(app));
    }
}
