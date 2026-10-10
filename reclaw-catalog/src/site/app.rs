//! An app as the catalog lists it (`/apps`, and the `entry` of an app's page): what it is, how a launcher installs it, its art, what
//! players said about it, and its newest and verified releases. Read leniently, like the rest of the site's answers.
use serde::Deserialize;
use serde_json::Value;

use super::types::{AiLevel, Verified, count, list, millis, text};

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GameRef {
    #[serde(deserialize_with = "text")]
    pub slug: String,
    #[serde(deserialize_with = "text")]
    pub title: String,
}

/// What a launcher needs to install the app: Quiver's list-entry fields, as the site keeps them.
#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Launcher {
    #[serde(deserialize_with = "text")]
    pub folder_name: String,
    pub release_asset_filter: Option<String>,
    /// Marker files made in the install folder (`portable.txt`).
    #[serde(deserialize_with = "list")]
    pub files_to_add: Vec<String>,
    /// The app's mods block, in the catalog lists' own shape (`path`, `layout`, `sources`); handed to the same reader as theirs.
    pub mods: Option<Value>,
}

/// SteamGridDB pictures the site picked for the app (or for the game it plays).
#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct LibraryArt {
    /// Portrait, 600x900.
    pub capsule: Option<String>,
    /// Landscape, 920x430.
    pub header: Option<String>,
    /// The wide banner behind a page's header.
    pub hero: Option<String>,
    pub logo: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Developer {
    #[serde(deserialize_with = "text")]
    pub name: String,
}

/// An app in the catalog (the API's `Entry`): what it is, what players said about it, and the newest release.
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SiteApp {
    #[serde(deserialize_with = "text")]
    pub id: String,
    /// The id the entry had in the community lists (`catalogId` there), for entries that came from them.
    pub catalog_id: Option<String>,
    #[serde(deserialize_with = "text")]
    pub slug: String,
    #[serde(deserialize_with = "text")]
    pub name: String,
    #[serde(deserialize_with = "text")]
    pub description: String,
    #[serde(deserialize_with = "text")]
    pub project_name: String,
    /// The original games it plays.
    #[serde(deserialize_with = "list")]
    pub games: Vec<GameRef>,
    /// The systems those games came out on, by the site's ids (`n64`, `ps2`, `gb`), the main one first.
    #[serde(deserialize_with = "list")]
    pub consoles: Vec<String>,
    #[serde(deserialize_with = "list")]
    pub tags: Vec<String>,
    /// The app's icon or cover.
    pub artwork: Option<String>,
    /// `artwork` is the game's, not the app's own: another port of the same game shows the same picture.
    pub artwork_from_game: bool,
    pub library_art: Option<LibraryArt>,
    pub launcher: Launcher,
    /// `port`, `tool`, `emulator` or `game`.
    #[serde(deserialize_with = "text")]
    pub project_type: String,
    /// `windows`, `linux`, `macos`, `android`, `ios`.
    #[serde(rename = "supportedOS", deserialize_with = "list")]
    pub supported_os: Vec<String>,
    pub ai_level: AiLevel,
    pub developer: Option<Developer>,
    /// Players who said it runs well.
    #[serde(deserialize_with = "count")]
    pub recommended: u32,
    #[serde(deserialize_with = "count")]
    pub review_count: u32,
    #[serde(deserialize_with = "count")]
    pub report_issues: u32,
    #[serde(deserialize_with = "count")]
    pub report_broken: u32,
    /// JavaScript milliseconds (they can have a fraction).
    #[serde(deserialize_with = "millis")]
    pub added_at: f64,
    /// The newest release upstream, verified or not.
    pub last_release_at: Option<f64>,
    pub last_release_version: Option<String>,
    pub verified: Option<Verified>,
}

impl SiteApp {
    /// The best picture of each shape: the site's library art, else the app's artwork (never a game's for an icon).
    pub fn capsule(&self) -> Option<&str> {
        self.library_art.as_ref().and_then(|a| a.capsule.as_deref()).filter(|u| !u.trim().is_empty())
    }

    pub fn hero(&self) -> Option<&str> {
        self.library_art.as_ref().and_then(|a| a.hero.as_deref()).filter(|u| !u.trim().is_empty())
    }

    /// The app's own icon: its artwork unless that was borrowed from the game.
    pub fn icon(&self) -> Option<&str> {
        self.artwork.as_deref().filter(|u| !self.artwork_from_game && !u.trim().is_empty())
    }

    /// The verified release's tag, if the site has verified one.
    pub fn verified_version(&self) -> Option<&str> {
        self.verified.as_ref().map(|v| v.version.trim()).filter(|v| !v.is_empty())
    }
}
