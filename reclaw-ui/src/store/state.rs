use std::collections::BTreeSet;

use reclaw_config::{LaunchPrefs, Preferences, WindowPrefs};
use reclaw_games::{project::ProjectInfo, settings::DisplayEnvironment};
use reclaw_input::ControllerInfo;

use crate::{
    activity::ActivityBoard,
    community::Community,
    credentials::CredentialsStatus,
    model::{GameEntry, ModEntry},
    nav::Route,
    notices::Notices,
    settings::{SettingsValues, persist},
};

use super::status::CatalogStatus;

/// One-shot messages from the host to the UI: set by the host, taken (cleared) by the UI.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Mailbox {
    pub open: Option<Route>,
}

/// Everything shared. Plain data: no handles, no Freya types, so the rules that change it are
/// tested as ordinary functions (see `reduce`).
#[derive(Clone, PartialEq, Debug)]
pub struct AppState {
    pub games: Vec<GameEntry>,
    /// What the catalog says about every project, installed or not.
    pub projects: Vec<ProjectInfo>,
    pub mods: Vec<ModEntry>,
    /// The installed games Reclaw can list and install mods for, by id, in the library's order. A game here may have no
    /// mods listed yet (its sites have not answered), which is why this is not derived from `mods`.
    pub moddable: Vec<u32>,
    /// What quiverlauncher.com and its players say about each game, and the game pages read from it.
    pub community: Community,
    /// Whether the catalog is loading, how fresh it is, and what went wrong with it.
    pub catalog: CatalogStatus,
    /// The access tokens' state (never the tokens). The host fills it in and keeps it current.
    pub credentials: CredentialsStatus,
    pub activity: ActivityBoard,
    pub notices: Notices,

    /// Reclaw's settings rows, for both interfaces. Saved.
    pub settings: SettingsValues,
    /// Launch defaults and per-game overrides. Saved.
    pub launch: LaunchPrefs,
    /// Games marked with the star. Saved; mirrored into each game's tags by the reducer.
    pub favorites: BTreeSet<u32>,
    pub window: WindowPrefs,

    pub controller: Option<ControllerInfo>,
    /// Height in px of the on-screen keyboard, 0 when hidden.
    pub keyboard_inset: f32,
    /// What the displays offer, for launch settings. The host fills it from the windowing system.
    pub display: DisplayEnvironment,
    pub mailbox: Mailbox,
    /// Why the saved settings were not used as they were, to tell the user once.
    pub settings_warning: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self::from_prefs(Preferences::default())
    }
}

impl AppState {
    /// The state at startup: empty library, the saved settings.
    pub fn from_prefs(prefs: Preferences) -> Self {
        Self {
            games: Vec::new(),
            projects: Vec::new(),
            mods: Vec::new(),
            moddable: Vec::new(),
            community: Community::default(),
            catalog: CatalogStatus::default(),
            credentials: CredentialsStatus::default(),
            activity: ActivityBoard::new(),
            notices: Notices::default(),
            settings: persist::from_prefs(&prefs.global, &prefs.apps),
            launch: prefs.launch,
            favorites: prefs.favorites,
            window: prefs.window,
            controller: None,
            keyboard_inset: 0.,
            display: DisplayEnvironment::unknown(),
            mailbox: Mailbox::default(),
            settings_warning: None,
        }
    }

    /// The part that is saved.
    pub fn to_prefs(&self) -> Preferences {
        let (global, apps) = persist::to_prefs(&self.settings);
        Preferences {
            version: Preferences::VERSION,
            global,
            apps,
            launch: self.launch.clone(),
            favorites: self.favorites.clone(),
            window: self.window.clone(),
        }
    }
}
