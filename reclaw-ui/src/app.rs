use freya::prelude::*;

use crate::{
    metrics::{Density, LayoutClass},
    model::{Download, GameEntry},
    screens::LibraryScreen,
    theme::{ThemeKind, use_init_reclaw, use_reclaw},
};

/// App root: provides the theme, measures its own width to pick the layout class, and renders the
/// Library page. `density` overrides the class default, e.g. `Touch` for a handheld at wide width.
#[derive(Clone, PartialEq)]
pub struct ReclawApp {
    pub games: Vec<GameEntry>,
    pub downloads: Vec<Download>,
    pub theme: ThemeKind,
    pub density: Option<Density>,
}

impl App for ReclawApp {
    fn render(&self) -> impl IntoElement {
        use_init_reclaw(self.theme);
        let t = use_reclaw();
        let mut class = use_state(LayoutClass::default);

        rect()
            .expanded()
            .background(t.bg_base)
            // set_if_modified: on_sized fires on every layout, an unconditional set would re-layout forever.
            .on_sized(move |e: Event<SizedEventData>| {
                class.set_if_modified(LayoutClass::from_width(e.area.width()))
            })
            .child(LibraryScreen::new(
                self.games.clone(),
                self.downloads.clone(),
                class(),
                self.density.unwrap_or_else(|| class().default_density()),
            ))
    }
}
