//! One component per route, in the order of the route table. Each only chooses between the
//! desktop and Deck version of its page and passes the route's parameters on; the pages
//! themselves are in `desktop::pages` and `deck::routes`.
use freya::prelude::*;
use reclaw_input::UiMode;

use crate::{
    deck::{routes as deck, settings::SettingsTarget},
    desktop::pages as desktop,
    nav::Section,
    shell::use_shell,
};

/// Build the page for whichever interface is showing. The shell read is a hook, so every page
/// calls this first and unconditionally.
fn by_mode(desktop: impl FnOnce() -> Element, deck: impl FnOnce() -> Element) -> Element {
    match use_shell().model.read().mode {
        UiMode::Desktop => desktop(),
        UiMode::Deck => deck(),
    }
}

#[derive(PartialEq)]
pub struct Library {}

impl Component for Library {
    fn render(&self) -> impl IntoElement {
        by_mode(|| desktop::LibraryPage {}.into_element(), || deck::HomeRoute { section: Section::Library }.into_element())
    }
}

#[derive(PartialEq)]
pub struct Catalog {}

impl Component for Catalog {
    fn render(&self) -> impl IntoElement {
        by_mode(|| desktop::CatalogPage {}.into_element(), || deck::HomeRoute { section: Section::Catalog }.into_element())
    }
}

#[derive(PartialEq)]
pub struct Game {
    pub id: u32,
}

impl Component for Game {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        by_mode(|| desktop::GamePage { id }.into_element(), || deck::GameRoute { id }.into_element())
    }
}

#[derive(PartialEq)]
pub struct Install {
    pub id: u32,
}

impl Component for Install {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        by_mode(|| desktop::InstallPage { id }.into_element(), || deck::InstallRoute { id }.into_element())
    }
}

#[derive(PartialEq)]
pub struct GameSettings {
    pub id: u32,
}

impl Component for GameSettings {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        by_mode(
            || desktop::GameSettingsPage { id, section: None }.into_element(),
            || deck::SettingsRoute { target: SettingsTarget::App(id) }.into_element(),
        )
    }
}

#[derive(PartialEq)]
pub struct GameSettingsSection {
    pub id: u32,
    pub section: String,
}

impl Component for GameSettingsSection {
    fn render(&self) -> impl IntoElement {
        let id = self.id;
        by_mode(
            || desktop::GameSettingsPage { id, section: Some(self.section.clone()) }.into_element(),
            || deck::SettingsRoute { target: SettingsTarget::App(id) }.into_element(),
        )
    }
}

#[derive(PartialEq)]
pub struct Mods {}

impl Component for Mods {
    fn render(&self) -> impl IntoElement {
        by_mode(|| desktop::ModsPage {}.into_element(), || deck::HomeRoute { section: Section::Mods }.into_element())
    }
}

#[derive(PartialEq)]
pub struct ModDetail {
    pub provider: String,
    pub mod_id: String,
}

impl Component for ModDetail {
    fn render(&self) -> impl IntoElement {
        by_mode(
            || desktop::ModDetailPage { provider: self.provider.clone(), mod_id: self.mod_id.clone() }.into_element(),
            || deck::ModDetailRoute { provider: self.provider.clone(), mod_id: self.mod_id.clone() }.into_element(),
        )
    }
}

#[derive(PartialEq)]
pub struct Downloads {}

impl Component for Downloads {
    fn render(&self) -> impl IntoElement {
        by_mode(|| desktop::DownloadsPage {}.into_element(), || deck::HomeRoute { section: Section::Downloads }.into_element())
    }
}

#[derive(PartialEq)]
pub struct Settings {}

impl Component for Settings {
    fn render(&self) -> impl IntoElement {
        by_mode(
            || desktop::SettingsPage { section: None }.into_element(),
            || deck::SettingsRoute { target: SettingsTarget::Global }.into_element(),
        )
    }
}

#[derive(PartialEq)]
pub struct SettingsSection {
    pub section: String,
}

impl Component for SettingsSection {
    fn render(&self) -> impl IntoElement {
        by_mode(
            || desktop::SettingsPage { section: Some(self.section.clone()) }.into_element(),
            || deck::SettingsRoute { target: SettingsTarget::Global }.into_element(),
        )
    }
}
