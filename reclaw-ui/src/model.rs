//! Data shapes from the contract's `dataModel`. Pure data: no Freya types except the art source.
use std::borrow::Cow;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppStatus {
    Installed,
    UpdateReady,
    Installing,
    Failed,
    Available,
    /// The project ships no copyrighted assets; the user must supply their own game file.
    NeedsFile,
}

impl AppStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::UpdateReady => "Update ready",
            Self::Installing => "Installing",
            Self::Failed => "Failed",
            Self::Available => "Not installed",
            Self::NeedsFile => "Needs your game file",
        }
    }

    pub fn is_installed(self) -> bool {
        matches!(self, Self::Installed | Self::UpdateReady)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    GitHub,
    GitLab,
    Manual,
}

impl Source {
    pub fn host(self) -> &'static str {
        match self {
            Self::GitHub => "github.com",
            Self::GitLab => "gitlab.com",
            Self::Manual => "local",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct GameEntry {
    pub id: u32,
    pub title: Cow<'static, str>,
    pub project: Cow<'static, str>,
    pub version: Cow<'static, str>,
    pub source: Source,
    pub status: AppStatus,
    pub tags: Vec<Cow<'static, str>>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DownloadStage {
    FetchingRelease,
    VerifyingHash,
    CheckingGameFile,
    Building,
    Extracting,
}

impl DownloadStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::FetchingRelease => "Fetching release",
            Self::VerifyingHash => "Verifying hash",
            Self::CheckingGameFile => "Checking your game file",
            Self::Building => "Building",
            Self::Extracting => "Extracting",
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Download {
    pub app_id: u32,
    pub title: Cow<'static, str>,
    pub stage: DownloadStage,
    /// 0.0 to 100.0
    pub progress: f32,
    pub detail: Cow<'static, str>,
    pub speed: Option<Cow<'static, str>>,
    /// Present when the download failed; shown instead of `detail`.
    pub error: Option<Cow<'static, str>>,
}
