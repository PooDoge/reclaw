use freya::prelude::*;
use reclaw_games::project::Platform;

use super::{dialogs::GameDialogs, pages::library::Filter};
use crate::{
    metrics::{Density, LayoutClass},
    model::ModProvider,
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
/// the search text and filter (go to a game and back and the list is as you left it), the library's
/// selected game, the Catalog and Mods chips, and the dialogs that any page can open.
#[derive(Clone, Copy)]
pub struct DesktopUi {
    pub env: State<DesktopEnv>,
    pub search: State<String>,
    pub filter: State<Filter>,
    pub selected: State<Option<u32>>,
    /// The Catalog's platform chip; `None` is All.
    pub platform: State<Option<Platform>>,
    /// The Mods list's provider chip; `None` is All.
    pub provider: State<Option<ModProvider>>,
    pub dialogs: GameDialogs,
}

/// The frame's shared state. Panics outside a `DesktopFrame`, which is a bug in the caller.
#[track_caller]
pub fn use_desktop_ui() -> DesktopUi {
    use_consume::<DesktopUi>()
}
