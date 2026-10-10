use freya::prelude::*;
use reclaw_games::project::Platform;

use super::{dialogs::GameDialogs, pages::library::Filter};
use crate::{
    metrics::{Density, LayoutClass},
    mod_shelves::OpenShelf,
    model::ModProvider,
    nav::{Nav, Route},
    search::{SearchModel, SearchScope},
    surface::SurfaceContext,
};

/// What every desktop page needs to know about the window it is in.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DesktopEnv {
    pub class: LayoutClass,
    pub density: Density,
    pub window: (f32, f32),
    /// Height of the on-screen keyboard, 0 when hidden.
    pub keyboard_inset: f32,
}

impl DesktopEnv {
    /// The form factor, which decides popups against full-screen pages and menu placement.
    pub fn surface(&self) -> SurfaceContext {
        SurfaceContext::new(self.class, self.density, self.window.1)
    }
}

/// State that belongs to the desktop frame rather than to one page, so it survives navigation:
/// each tab's search and the filter (go to a game and back and the list is as you left it), the library's
/// selected game, the Catalog and Mods chips, and the dialogs that any page can open.
#[derive(Clone, Copy)]
pub struct DesktopUi {
    pub env: State<DesktopEnv>,
    /// Every tab's search, and whether the search box is open (`search::SearchModel`).
    pub search: State<SearchModel>,
    /// The text in the search box while it is open.
    pub search_field: State<String>,
    /// The Mods tab's opened shelf and its page; `None` shows every shelf.
    pub mod_shelf: State<Option<OpenShelf>>,
    /// The page of the Mods tab's search results.
    pub mod_results_page: State<usize>,
    pub filter: State<Filter>,
    pub selected: State<Option<u32>>,
    /// The Library's system filter; `None` is all systems.
    pub system: State<Option<Platform>>,
    /// The Catalog's platform chip; `None` is All.
    pub platform: State<Option<Platform>>,
    /// The Mods list's provider chip; `None` is All.
    pub provider: State<Option<ModProvider>>,
    /// The Mods list's game, by id; `None` is all games. Set by the game chip and by a Game page's link.
    pub mod_game: State<Option<u32>>,
    pub dialogs: GameDialogs,
}

impl DesktopUi {
    /// Open the Mods tab on one game's mods: every site, no search, so what it shows is everything listed for the game.
    pub fn show_mods_for(mut self, nav: Nav, game: u32) {
        self.mod_game.set(Some(game));
        self.provider.set(None);
        self.mod_shelf.set(None);
        self.search.write().clear(SearchScope::Mods);
        nav.open(Route::Mods {});
    }

    /// The search a tab's results follow; empty when none is running.
    pub fn query(&self, scope: SearchScope) -> String {
        self.search.read().query(scope).to_string()
    }

    /// End a tab's search, from a results line's Clear button.
    pub fn clear_search(mut self, scope: SearchScope) {
        self.search.write().clear(scope);
        if scope == SearchScope::Mods {
            self.mod_results_page.set(0);
        }
    }
}

/// The frame's shared state. Panics outside a `DesktopFrame`, which is a bug in the caller.
#[track_caller]
pub fn use_desktop_ui() -> DesktopUi {
    use_consume::<DesktopUi>()
}
