//! Deck's page and the router's route, kept in step. The router is the truth about where the app
//! is; the reducer keeps a copy (its screen and tab) because focus and the back stack hang off it,
//! and asks the router to move whenever it moves itself. When something else moves the router (the
//! mouse's back button, a deep link, switching in from the desktop) the reducer follows.
use super::{DeckState, DeckView, Effect, Screen, Section, SettingsTarget};
use crate::nav::Route;

impl DeckState {
    /// The route that matches what Deck is showing.
    pub fn route(&self) -> Route {
        match self.screen {
            Screen::Home => Route::of_section(self.section),
            Screen::Game(id) => Route::Game { id },
            Screen::Install(id) => Route::Install { id },
            Screen::Settings(SettingsTarget::Global) => Route::Settings {},
            Screen::Settings(SettingsTarget::App(id)) => Route::GameSettings { id },
        }
    }

    /// Show `route` because the router is there. Does nothing when Deck already shows it, which is
    /// the usual case: the reducer moved first and the router followed. Returns whether it changed.
    pub fn follow(&mut self, route: &Route, view: &DeckView) -> bool {
        if *route == self.route() {
            return false;
        }
        let (screen, section) = match route {
            Route::Library {} | Route::Catalog {} | Route::Mods {} | Route::Downloads {} => {
                (Screen::Home, route.section().unwrap_or(self.section))
            }
            // Deck has no page for one mod: its tab is the nearest thing.
            Route::ModDetail { .. } => (Screen::Home, Section::Mods),
            Route::Game { id } => (Screen::Game(*id), self.section),
            Route::Install { id } => (Screen::Install(*id), self.section),
            Route::Settings {} | Route::SettingsSection { .. } => (Screen::Settings(SettingsTarget::Global), self.section),
            Route::GameSettings { id } | Route::GameSettingsSection { id, .. } => {
                (Screen::Settings(SettingsTarget::App(*id)), self.section)
            }
        };
        // A page the library does not have (a stale link): stay on the tab.
        let missing = match screen {
            Screen::Game(id) | Screen::Install(id) | Screen::Settings(SettingsTarget::App(id)) => view.game(id).is_none(),
            _ => false,
        };
        let (screen, section) = if missing { (Screen::Home, self.section) } else { (screen, section) };

        self.leave_scope();
        self.overlay = super::Overlay::None;
        self.menu = None;
        self.drilled = false;
        self.settings_section = 0;
        // Whatever led here is no longer the way back: the router decides that now.
        self.stack.clear();
        self.screen = screen;
        self.section = section;
        self.enter_scope(view);
        true
    }

    /// Ask the router to show what Deck now shows. Called after every move the reducer makes.
    pub(super) fn request_route(&mut self) {
        let route = self.route();
        self.nav_requests.push(Effect::Navigate(route));
    }

    /// Ask the router to go back one page.
    pub(super) fn request_back(&mut self) {
        self.nav_requests.push(Effect::Back);
    }

    /// Hand the router requests made since the last call to the caller's effect list.
    pub(super) fn drain_nav(&mut self, fx: &mut Vec<Effect>) {
        fx.append(&mut self.nav_requests);
    }
}
