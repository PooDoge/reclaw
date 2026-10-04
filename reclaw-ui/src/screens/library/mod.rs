//! The Library page at all three layout classes. Wide: top nav + sidebar + hero. Compact: icon
//! rail + hero + capsule grid. Phone: capsule grid, a pushed game page and bottom tabs.
//!
//! * `filter`: which games show (pure, tested); `ctx`: what the layouts share;
//! * `wide` / `compact` / `phone`: one file per layout; `parts`: pieces more than one uses;
//! * the install dialog and the Manage menu are adaptive surfaces (see `crate::surface`).
mod compact;
mod ctx;
mod filter;
mod install_form;
mod parts;
mod phone;
mod wide;

use freya::prelude::*;

use self::{ctx::Ctx, filter::Filter, install_form::InstallForm};
use crate::{app_menu::MenuAction, metrics::*, prelude::*};

#[derive(Clone, PartialEq)]
pub struct LibraryScreen {
    games: Vec<GameEntry>,
    downloads: Vec<Download>,
    class: LayoutClass,
    density: Density,
    window: (f32, f32),
    keyboard_inset: f32,
    on_action: Option<EventHandler<(u32, MenuAction)>>,
    on_deck_mode: Option<EventHandler<()>>,
}

impl LibraryScreen {
    pub fn new(games: Vec<GameEntry>, downloads: Vec<Download>, class: LayoutClass, density: Density) -> Self {
        let window = match class {
            LayoutClass::Wide => (1100., 700.),
            LayoutClass::Compact => (860., 640.),
            LayoutClass::Phone => (390., 780.),
        };
        Self { games, downloads, class, density, window, keyboard_inset: 0., on_action: None, on_deck_mode: None }
    }

    /// The window size in px. Dialogs choose popup or full screen from it.
    pub fn window(mut self, window: (f32, f32)) -> Self {
        self.window = window;
        self
    }

    /// Height of the on-screen keyboard, 0 when hidden.
    pub fn keyboard_inset(mut self, inset: f32) -> Self {
        self.keyboard_inset = inset;
        self
    }

    /// An Options menu choice for the selected app: `(app id, action)`.
    pub fn on_action(mut self, handler: impl Into<EventHandler<(u32, MenuAction)>>) -> Self {
        self.on_action = Some(handler.into());
        self
    }

    /// Switch to Deck mode. Shown as a button in the top bar when set.
    pub fn on_deck_mode(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_deck_mode = Some(handler.into());
        self
    }
}

impl Component for LibraryScreen {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let selected = use_state(|| self.games.first().map(|g| g.id));
        let page_open = use_state(|| false);
        let filter = use_state(|| Filter::All);
        let search = use_state(String::new);
        let nav = use_state(|| NavItem::Library);
        let dialog_open = use_state(|| false);
        let manage = use_state(|| None);
        let confirm = use_state(|| None);
        let form = InstallForm::use_new();

        let ctx = Ctx {
            t,
            visible: filter::visible(&self.games, filter(), &search.read()),
            current: self.games.iter().find(|g| Some(g.id) == selected()).cloned(),
            installed: self.games.iter().filter(|g| g.status.is_installed()).count() as u32,
            updates: self.games.iter().filter(|g| g.status == AppStatus::UpdateReady).count() as u32,
            games: self.games.clone(),
            downloads: self.downloads.clone(),
            class: self.class,
            density: self.density,
            window: self.window,
            keyboard_inset: self.keyboard_inset,
            selected,
            page_open,
            filter,
            search,
            nav,
            dialog_open,
            confirm,
            form,
            manage,
            on_action: self.on_action.clone(),
            on_deck_mode: self.on_deck_mode.clone(),
        };

        let page = match self.class {
            LayoutClass::Wide => wide::layout(&ctx),
            LayoutClass::Compact => compact::layout(&ctx),
            LayoutClass::Phone => phone::layout(&ctx),
        };
        rect()
            .expanded()
            .child(page)
            .child(parts::install_dialog(&ctx))
            .maybe_child(parts::manage_menu(&ctx))
            .maybe_child(parts::uninstall_confirm(&ctx))
    }
}
