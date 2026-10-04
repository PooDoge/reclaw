use std::time::Duration;

use crate::model::ModProvider;

/// Identifies one job. Chosen by the host, unique for the life of the process.
pub type ActivityId = u64;

/// What the job is for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Kind {
    /// A first install of a game.
    Install,
    /// A new version of an installed game.
    Update,
    /// A mod for a game.
    Mod { provider: ModProvider, id: String },
}

/// Where the job is. The recompilation pipeline's steps, then the mod download's.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Stage {
    Queued,
    Downloading,
    Verifying,
    CheckingGameFile,
    Building,
    Extracting,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Downloading => "Downloading",
            Self::Verifying => "Verifying",
            Self::CheckingGameFile => "Checking your game file",
            Self::Building => "Building",
            Self::Extracting => "Extracting",
        }
    }

    /// Bytes are still coming in. After this the job is working on files it already has.
    pub fn is_transfer(self) -> bool {
        matches!(self, Self::Queued | Self::Downloading)
    }
}

/// What release notes to show after an update.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Changelog {
    pub from: String,
    pub to: String,
    /// Plain text; the first lines are shown, the rest is a link away.
    pub notes: String,
    pub url: Option<String>,
}

impl Changelog {
    /// The non-empty lines of the notes, for a notification's details.
    pub fn lines(&self) -> Vec<&str> {
        self.notes.lines().map(str::trim).filter(|l| !l.is_empty()).collect()
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Outcome {
    Running,
    Finished,
    Failed { reason: String },
    Cancelled,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Activity {
    pub id: ActivityId,
    pub game_id: u32,
    pub kind: Kind,
    /// "Skyward Quest v0.9.2", "HD Texture Pack".
    pub title: String,
    pub stage: Stage,
    pub bytes_done: u64,
    pub bytes_total: Option<u64>,
    /// Bytes per second, as smoothed by the host.
    pub rate: Option<u64>,
    /// Set when an update finished; what the sidebar offers to read.
    pub changelog: Option<Changelog>,
    pub outcome: Outcome,
}

impl Activity {
    pub fn is_running(&self) -> bool {
        self.outcome == Outcome::Running
    }

    pub fn is_mod(&self) -> bool {
        matches!(self.kind, Kind::Mod { .. })
    }

    /// 0 to 1 through the transfer, when the size is known. Finished is 1.
    pub fn progress(&self) -> Option<f32> {
        if self.outcome == Outcome::Finished {
            return Some(1.);
        }
        match self.bytes_total {
            Some(total) if total > 0 => Some((self.bytes_done as f64 / total as f64).clamp(0., 1.) as f32),
            _ => None,
        }
    }

    /// Whole percent for display. Never 100 while running: "100%" followed by minutes of work reads as stuck.
    pub fn percent(&self) -> Option<u8> {
        let p = (self.progress()? * 100.).floor() as u8;
        Some(if self.is_running() { p.min(99) } else { p })
    }

    /// Time left in the transfer, if the size and speed are known. Not offered after the transfer
    /// stage: the work that follows has no honest estimate.
    pub fn eta(&self) -> Option<Duration> {
        if !self.is_running() || !self.stage.is_transfer() {
            return None;
        }
        let (total, rate) = (self.bytes_total?, self.rate.filter(|r| *r > 0)?);
        let left = total.saturating_sub(self.bytes_done);
        Some(Duration::from_secs(left.div_ceil(rate)))
    }
}
