use freya::prelude::*;

use super::app::DeckApp;
use crate::shell::use_shell;

/// Deck mode's frame inside the router. For now it hosts the existing `DeckApp`, which still keeps
/// its own page state; routes take over from it page by page.
#[derive(PartialEq)]
pub struct DeckFrame {}

impl Component for DeckFrame {
    fn render(&self) -> impl IntoElement {
        let shell = use_shell();
        DeckApp {
            store: shell.store,
            feed: shell.feed.clone(),
            on_effect: shell.on_effect.clone(),
            map: shell.map.clone(),
            script: shell.script.clone(),
        }
    }
}
