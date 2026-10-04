use serde::{Deserialize, Serialize};

use super::value::Size;

/// The windowing system the game will run under. It decides what a game *can* be asked to do:
/// Wayland compositors own fullscreen and monitor placement, gamescope owns everything.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayServer {
    Wayland,
    X11,
    /// Steam Gaming Mode / Bazzite Deck: a single fullscreen compositor.
    Gamescope,
    Windows,
    MacOs,
    Unknown,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Monitor {
    /// Stable id the host can resolve again (a connector name like `DP-1`, or an index).
    pub id: String,
    pub name: String,
    pub native: Size,
    pub refresh_mhz: u32,
    pub primary: bool,
}

/// What the host tells us about the display. The UI crate never probes the system itself.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DisplayEnvironment {
    pub server: DisplayServer,
    pub monitors: Vec<Monitor>,
}

impl DisplayEnvironment {
    /// A single unnamed screen of unknown kind: what tests and the gallery use.
    pub fn unknown() -> Self {
        Self { server: DisplayServer::Unknown, monitors: Vec::new() }
    }

    pub fn primary(&self) -> Option<&Monitor> {
        self.monitors.iter().find(|m| m.primary).or_else(|| self.monitors.first())
    }
}
