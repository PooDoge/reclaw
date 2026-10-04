//! What the catalog says about a recompilation project. A [`ProjectInfo`] is the same for every
//! user; what *this* user has installed lives in the library (`GameEntry` in `reclaw-ui`).
use serde::{Deserialize, Serialize};

use crate::settings::Capabilities;

/// The console the original game ran on.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    N64,
    Ps2,
    Gba,
    Other,
}

impl Platform {
    pub fn label(self) -> &'static str {
        match self {
            Self::N64 => "Nintendo 64",
            Self::Ps2 => "PlayStation 2",
            Self::Gba => "Game Boy Advance",
            Self::Other => "Other",
        }
    }
}

/// Where releases come from.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RepoRef {
    pub host: RepoHost,
    pub owner: String,
    pub name: String,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoHost {
    Github,
    Gitlab,
}

impl RepoRef {
    pub fn url(&self) -> String {
        let host = match self.host {
            RepoHost::Github => "github.com",
            RepoHost::Gitlab => "gitlab.com",
        };
        format!("https://{host}/{}/{}", self.owner, self.name)
    }
}

/// An image or video shown on the game page. `url` may be remote or a path in the catalog cache;
/// the UI loads it lazily and shows a placeholder until it arrives.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Media {
    Screenshot {
        url: String,
        caption: Option<String>,
    },
    /// Videos open in the system player or browser; Freya has no video widget in this release.
    Video {
        url: String,
        thumbnail: Option<String>,
        title: Option<String>,
    },
}

/// One entry of the project's release history: the "recent updates" shown on the game page.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Release {
    pub tag: String,
    pub name: String,
    /// ISO date, `2026-09-30`. Kept as text: the UI only shows it and sorts by list order.
    pub published: String,
    /// Release notes as plain text; the UI shows the first lines.
    pub notes: String,
    pub url: String,
    pub prerelease: bool,
}

/// Hardware a game needs. Every field is optional because projects publish what they have.
#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct SpecSheet {
    pub os: Option<String>,
    pub cpu: Option<String>,
    pub gpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
}

impl SpecSheet {
    /// `(label, value)` for the lines that are present, in reading order.
    pub fn lines(&self) -> Vec<(&'static str, &str)> {
        [("OS", &self.os), ("Processor", &self.cpu), ("Graphics", &self.gpu), ("Memory", &self.memory), ("Storage", &self.storage)]
            .into_iter()
            .filter_map(|(label, value)| value.as_deref().map(|v| (label, v)))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.lines().is_empty()
    }
}

#[derive(Clone, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Requirements {
    pub minimum: SpecSheet,
    pub recommended: Option<SpecSheet>,
    pub notes: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub id: u32,
    pub title: String,
    pub summary: String,
    pub description: String,
    pub platform: Platform,
    pub repo: RepoRef,
    /// Banner behind the page header and the tint source for Deck's Big Art. Optional: a placeholder
    /// is drawn without it.
    pub hero_url: Option<String>,
    pub capsule_url: Option<String>,
    pub media: Vec<Media>,
    pub releases: Vec<Release>,
    pub requirements: Option<Requirements>,
    pub tags: Vec<String>,
    /// Launch settings this game can honor. Empty means the game page shows none.
    pub capabilities: Capabilities,
}

impl ProjectInfo {
    pub fn latest_release(&self) -> Option<&Release> {
        self.releases.iter().find(|r| !r.prerelease).or_else(|| self.releases.first())
    }

    pub fn screenshots(&self) -> impl Iterator<Item = &Media> {
        self.media.iter().filter(|m| matches!(m, Media::Screenshot { .. }))
    }

    pub fn videos(&self) -> impl Iterator<Item = &Media> {
        self.media.iter().filter(|m| matches!(m, Media::Video { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sheet() -> SpecSheet {
        SpecSheet { os: Some("Linux or Windows".into()), memory: Some("4 GB".into()), ..SpecSheet::default() }
    }

    #[test]
    fn spec_lines_skip_what_is_missing_and_keep_reading_order() {
        assert_eq!(sheet().lines(), vec![("OS", "Linux or Windows"), ("Memory", "4 GB")]);
        assert!(SpecSheet::default().is_empty());
    }

    #[test]
    fn repo_urls_follow_the_host() {
        let github = RepoRef { host: RepoHost::Github, owner: "a".into(), name: "b".into() };
        let gitlab = RepoRef { host: RepoHost::Gitlab, ..github.clone() };
        assert_eq!(github.url(), "https://github.com/a/b");
        assert_eq!(gitlab.url(), "https://gitlab.com/a/b");
    }
}
