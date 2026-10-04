//! The desktop interface: the Library page at the layout class and density the window calls for.
use freya::prelude::*;

use crate::{
    app_menu::MenuAction,
    host::HostState,
    metrics::{Density, LayoutClass},
    prelude::*,
    screens::LibraryScreen,
};

/// Desktop root content. It does not set up the theme (the app root does, once), so the `Shell`
/// can show it and Deck mode over the same theme.
///
/// `layout` and `density` override what the window width would pick: `density: Some(Touch)` for a
/// handheld in desktop mode, `layout: Some(Phone)` to see the phone layout in a wide window.
#[derive(Clone, PartialEq)]
pub struct DesktopApp {
    pub host: HostState,
    pub layout: Option<LayoutClass>,
    pub density: Option<Density>,
    pub on_action: Option<EventHandler<(u32, MenuAction)>>,
    /// Offered as a "Deck mode" button when set.
    pub on_deck_mode: Option<EventHandler<()>>,
}

impl DesktopApp {
    pub fn new(host: HostState) -> Self {
        Self { host, layout: None, density: None, on_action: None, on_deck_mode: None }
    }
}

impl Component for DesktopApp {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let mut window = use_state(|| (1100.0f32, 700.0f32));
        let class = self.layout.unwrap_or_else(|| LayoutClass::from_width(window().0));
        let density = self.density.unwrap_or_else(|| class.default_density());

        let mut screen = LibraryScreen::new(self.host.games.read().clone(), self.host.downloads.read().clone(), class, density)
            .window(window())
            .keyboard_inset(*self.host.keyboard_inset.read());
        if let Some(h) = &self.on_action {
            screen = screen.on_action(h.clone());
        }
        if let Some(h) = &self.on_deck_mode {
            screen = screen.on_deck_mode(h.clone());
        }
        rect()
            .expanded()
            .background(t.bg_base)
            // set_if_modified: on_sized fires on every layout, an unconditional set would re-layout forever.
            .on_sized(move |e: Event<SizedEventData>| window.set_if_modified((e.area.width(), e.area.height())))
            .child(screen)
    }
}
