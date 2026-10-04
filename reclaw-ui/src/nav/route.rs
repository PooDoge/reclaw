//! Every page of the app, as one enum. `freya-router` derives the path, parsing and the site map
//! from it, so a route can be a link, a saved location or a command-line argument (`--open
//! /game/4`). The variant names the page; `pages::*` are the components that draw it.
//!
//! Pages are leaves under one layout: `AppLayout` is the persistent shell (navigation, overlays,
//! the stage that animates between pages). There are no nested layouts on purpose: a section such
//! as Mods is a set of sibling routes, so a transition never has to animate a layout around an
//! outlet, and Back is always "the previous route".
use freya::router::Routable;

use crate::pages;

#[derive(Routable, Clone, PartialEq, Eq, Hash, Debug)]
#[rustfmt::skip]
pub enum Route {
    #[layout(pages::AppLayout)]
        /// The user's library: installed and added apps.
        #[route("/", pages::Library)]
        Library {},
        /// Every known recompilation project.
        #[route("/catalog", pages::Catalog)]
        Catalog {},
        /// One project: media, updates, requirements, play or install, favorite, settings.
        #[route("/game/:id", pages::Game)]
        Game { id: u32 },
        /// Where to install, which of the user's own files to build from.
        #[route("/game/:id/install", pages::Install)]
        Install { id: u32 },
        /// Launch settings for one game: only the ones the game supports.
        #[route("/game/:id/settings", pages::GameSettings)]
        GameSettings { id: u32 },
        #[route("/game/:id/settings/:section", pages::GameSettingsSection)]
        GameSettingsSection { id: u32, section: String },
        #[route("/mods", pages::Mods)]
        Mods {},
        #[route("/mods/:provider/:mod_id", pages::ModDetail)]
        ModDetail { provider: String, mod_id: String },
        #[route("/downloads", pages::Downloads)]
        Downloads {},
        /// Reclaw's own settings, including the default launch settings for games.
        #[route("/settings", pages::Settings)]
        Settings {},
        #[route("/settings/:section", pages::SettingsSection)]
        SettingsSection { section: String },
}
