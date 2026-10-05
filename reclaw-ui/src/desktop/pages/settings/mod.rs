//! The settings pages: Reclaw's own (`/settings`) and one game's properties (`/game/:id/settings`).
//! Both show a schema from `crate::settings`, the same one Deck mode shows, so a row added there
//! appears in both interfaces. Launch rows appear only where the game and the display support them.
//!
//! * `screen`: the section list and the rows, at each layout class; `form`: one section's rows
//! * `text_row`: a text row, which owns its field; `tokens`: the boxes access tokens are pasted into
mod form;
mod screen;
mod text_row;
mod tokens;

use freya::prelude::*;

use self::screen::{GameSource, SettingsScreen};
use crate::settings::SettingsTarget;

/// Reclaw's settings. `section` is the section named by the URL.
#[derive(PartialEq)]
pub struct SettingsPage {
    pub section: Option<String>,
}

impl Component for SettingsPage {
    fn render(&self) -> impl IntoElement {
        SettingsScreen { target: SettingsTarget::Global, section: self.section.clone(), source: GameSource::None }
    }
}

/// One game's properties: its launch settings (if it has any), updates, files.
#[derive(PartialEq)]
pub struct GameSettingsPage {
    pub id: u32,
    pub section: Option<String>,
}

impl Component for GameSettingsPage {
    fn render(&self) -> impl IntoElement {
        SettingsScreen { target: SettingsTarget::App(self.id), section: self.section.clone(), source: GameSource::Library(self.id) }
    }
}
