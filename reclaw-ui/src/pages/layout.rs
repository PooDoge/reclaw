use freya::prelude::*;
use reclaw_input::UiMode;

use crate::{
    deck::DeckFrame,
    desktop::DesktopFrame,
    nav::{Nav, NavInputExt},
    shell::use_shell,
};

/// The persistent shell around every page: the navigation handle, the input that goes back and
/// forward, and whichever interface is showing. Router calls it with no props, so everything it
/// needs comes from the shell's context.
#[derive(PartialEq)]
pub struct AppLayout {}

impl Component for AppLayout {
    fn render(&self) -> impl IntoElement {
        let nav = Nav::use_provide();
        let shell = use_shell();
        let mut open = shell.host.open;
        // The host asks for a page by writing it here; take it, clear it, go.
        use_side_effect(move || {
            let requested = open.read().clone();
            if let Some(route) = requested {
                open.set(None);
                nav.open(route);
            }
        });
        let mode = shell.model.read().mode;
        let frame = match mode {
            UiMode::Desktop => DesktopFrame {}.into_element(),
            UiMode::Deck => DeckFrame {}.into_element(),
        };
        // Keyed by mode so switching interfaces gives each a fresh scope.
        rect().expanded().nav_input(nav, mode == UiMode::Desktop).child(rect().expanded().key(format!("{mode:?}")).child(frame))
    }
}
