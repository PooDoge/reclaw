//! Data shapes from the contract's `dataModel`. Pure data: no Freya types except the art source.
use std::borrow::Cow;

use reclaw_games::project::{Platform, ProjectInfo, RepoHost};

pub use reclaw_runtime::RunState;

/// Where a game's pictures are on the internet. A missing one leaves a placeholder.
#[derive(Clone, Default, PartialEq, Eq, Debug)]
pub struct Art {
    pub capsule: Option<String>,
    pub hero: Option<String>,
    /// Where more pictures can be found when `hero` is missing: the repository's README (see `components::BannerArt`).
    pub repo: Option<reclaw_games::project::RepoRef>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AppStatus {
    Installed,
    UpdateReady,
    Installing,
    Failed,
    Available,
}

impl AppStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::UpdateReady => "Update ready",
            Self::Installing => "Installing",
            Self::Failed => "Failed",
            Self::Available => "Not installed",
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
    /// The system the game was recompiled from, for the badge and for browsing by system.
    pub platform: Platform,
    pub art: Art,
    /// Whether the user has added it to their library (as opposed to a catalog project they are only looking at).
    pub in_library: bool,
    /// Whether the app is running right now. Independent of `status`, which is install state.
    pub run: RunState,
}

impl GameEntry {
    /// A project the user has not added: shown in the catalog as available to install.
    pub fn from_project(project: &ProjectInfo) -> Self {
        Self {
            id: project.id,
            title: Cow::Owned(project.title.clone()),
            project: Cow::Owned(if project.project.is_empty() { project.platform.label().to_string() } else { project.project.clone() }),
            version: Cow::Owned(project.latest_release().map(|r| r.tag.clone()).unwrap_or_default()),
            source: match project.repo.host {
                RepoHost::Github => Source::GitHub,
                RepoHost::Gitlab => Source::GitLab,
            },
            status: AppStatus::Available,
            tags: project.tags.iter().cloned().map(Cow::Owned).collect(),
            platform: project.platform,
            art: Art { capsule: project.capsule_url.clone(), hero: project.hero_url.clone(), repo: Some(project.repo.clone()) },
            in_library: false,
            run: RunState::Idle,
        }
    }

    pub fn is_favorite(&self) -> bool {
        self.tags.iter().any(|t| t == "favorite")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ModProvider {
    Thunderstore,
    GameBanana,
}

impl ModProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Thunderstore => "Thunderstore",
            Self::GameBanana => "GameBanana",
        }
    }

    /// The path segment in `/mods/:provider/:mod_id`.
    pub fn slug(self) -> &'static str {
        match self {
            Self::Thunderstore => "thunderstore",
            Self::GameBanana => "gamebanana",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        [Self::Thunderstore, Self::GameBanana].into_iter().find(|p| p.slug() == slug)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModStatus {
    Available,
    Installing,
    Installed,
    /// Installed, and the site has a newer version.
    UpdateReady,
}

impl ModStatus {
    pub fn is_installed(self) -> bool {
        matches!(self, Self::Installed | Self::UpdateReady)
    }
}

/// A mod for one game, from a mod site. Like `GameEntry`, it carries the user's state (`status`).
#[derive(Clone, PartialEq, Debug)]
pub struct ModEntry {
    pub provider: ModProvider,
    pub id: String,
    pub game_id: u32,
    pub title: String,
    pub author: String,
    pub summary: String,
    /// The newest version the site lists (the installed one when the site does not say).
    pub version: String,
    /// The version in the game's folder, when installed.
    pub installed_version: Option<String>,
    pub downloads: u64,
    pub tags: Vec<String>,
    pub status: ModStatus,
    /// The mod's picture on its site.
    pub icon: Option<String>,
    /// The mod's page on its site.
    pub page_url: Option<String>,
}

impl ModEntry {
    pub fn matches(&self, provider: &str, id: &str) -> bool {
        self.provider.slug() == provider && self.id == id
    }
}
