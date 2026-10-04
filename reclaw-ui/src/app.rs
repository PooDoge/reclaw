use freya::prelude::*;

use crate::{
    desktop::DesktopApp,
    host::HostState,
    metrics::Density,
    model::{Download, GameEntry},
    theme::{ThemeKind, use_init_reclaw},
};

/// A desktop-only app root for the gallery and snapshots: provides the theme and shows the
/// Library page. `density` overrides the class default, e.g. `Touch` for a handheld at wide width.
/// The real app root is [`Shell`](crate::shell::Shell), which can also show Deck mode.
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
        let host = HostState::use_new(self.games.clone(), self.downloads.clone());
        DesktopApp { density: self.density, ..DesktopApp::new(host) }
    }
}
