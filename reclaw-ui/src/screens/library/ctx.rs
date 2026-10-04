//! What the three Library layouts share: the derived lists, the page's state handles and the
//! window facts. Built once per render by `LibraryScreen`.
use freya::prelude::*;

use super::{filter::Filter, install_form::InstallForm};
use crate::{
    app_menu::MenuAction,
    metrics::*,
    prelude::*,
    surface::{MenuState, SurfaceContext},
};

/// An open Manage menu and where it was pressed.
pub(super) type OpenMenu = (MenuState<MenuAction>, (f32, f32));

pub(super) struct Ctx {
    /// Tokens, read once in `LibraryScreen::render`: hooks must not run inside the layout helpers,
    /// which differ between layout classes and renders.
    pub t: Reclaw,
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Download>,
    /// After the filter chips and the search box.
    pub visible: Vec<GameEntry>,
    pub current: Option<GameEntry>,
    pub class: LayoutClass,
    pub density: Density,
    pub window: (f32, f32),
    pub keyboard_inset: f32,
    pub installed: u32,
    pub updates: u32,
    pub selected: State<Option<u32>>,
    pub page_open: State<bool>,
    pub filter: State<Filter>,
    pub search: State<String>,
    pub nav: State<NavItem>,
    pub dialog_open: State<bool>,
    /// The app awaiting an uninstall confirmation.
    pub confirm: State<Option<u32>>,
    pub form: InstallForm,
    /// The open Manage menu and where it was pressed.
    pub manage: State<Option<OpenMenu>>,
    pub on_action: Option<EventHandler<(u32, MenuAction)>>,
    pub on_deck_mode: Option<EventHandler<()>>,
}

impl Ctx {
    pub fn surface(&self) -> SurfaceContext {
        SurfaceContext::new(self.class, self.density, self.window.1)
    }
}
