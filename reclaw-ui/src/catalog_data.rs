//! From what the catalog says and what the user keeps, to what the screens show: `ProjectInfo`s for the Catalog and the game page,
//! `GameEntry`s for the Library. Pure, so the mapping is tested on plain data and against the real catalog.
//!
//! The screens identify a game by a `u32` that is saved (favorites, per-game settings), so it must be the same on every start and
//! must not depend on the order the catalog lists things: [`IdMap`] derives it from the app's identity.
use std::{borrow::Cow, collections::BTreeMap};

use reclaw_catalog::{
    AppEntry,
    site::{AiLevel, SiteApp},
};
use reclaw_games::{
    listing::{AiUse, Listing, Ratings},
    project::{Platform, ProjectInfo, Release, RepoHost, RepoRef},
};
use reclaw_runtime::RunState;
use reclaw_sync::CatalogApp;

use crate::model::{AppStatus, Art, GameEntry, Source};

/// 32-bit FNV-1a, which (unlike `DefaultHasher`) is specified and so gives the same number in every build.
fn fnv1a(text: &str) -> u32 {
    text.bytes().fold(0x811c_9dc5u32, |hash, byte| (hash ^ u32::from(byte)).wrapping_mul(0x0100_0193))
}

/// What names an app in the saved data: its tile key, upper-cased because keys compare ignoring case.
pub fn key_of(entry: &AppEntry) -> String {
    entry.instance_key().to_uppercase()
}

/// Each app's number. A number is the hash of the key; the rare two keys with one hash are told apart by giving the one that
/// sorts later the next free number, so the result depends on which keys exist and never on the order they arrived in.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct IdMap(BTreeMap<String, u32>);

impl IdMap {
    pub fn for_keys(keys: impl IntoIterator<Item = String>) -> Self {
        let mut keys: Vec<String> = keys.into_iter().collect();
        keys.sort();
        keys.dedup();
        let mut taken = std::collections::HashSet::new();
        let mut map = BTreeMap::new();
        for key in keys {
            let mut id = fnv1a(&key);
            while !taken.insert(id) {
                id = id.wrapping_add(1);
            }
            map.insert(key, id);
        }
        Self(map)
    }

    pub fn get(&self, key: &str) -> Option<u32> {
        self.0.get(key).copied()
    }
}

/// What the screens are given.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Loaded {
    pub projects: Vec<ProjectInfo>,
    pub games: Vec<GameEntry>,
}

/// `owner/name`. A GitLab project can sit in nested groups (`group/sub/project`): the owner is everything before the last slash.
fn repo_ref(entry: &AppEntry) -> RepoRef {
    let (owner, name) = entry.repository.trim().rsplit_once('/').unwrap_or(("", entry.repository.trim()));
    let host = match entry.source {
        reclaw_catalog::RepoSource::Github => RepoHost::Github,
        reclaw_catalog::RepoSource::Gitlab => RepoHost::Gitlab,
    };
    RepoRef { host, owner: owner.to_string(), name: name.to_string() }
}

fn release_url(repo: &RepoRef, tag: &str) -> String {
    match repo.host {
        RepoHost::Github => format!("https://github.com/{}/{}/releases/tag/{tag}", repo.owner, repo.name),
        RepoHost::Gitlab => format!("https://gitlab.com/{}/{}/-/releases/{tag}", repo.owner, repo.name),
    }
}

fn source_of(entry: &AppEntry) -> Source {
    if entry.is_manual() {
        Source::Manual
    } else if entry.source == reclaw_catalog::RepoSource::Gitlab {
        Source::GitLab
    } else {
        Source::GitHub
    }
}

/// JavaScript milliseconds as Unix seconds; nothing for zero or less.
fn seconds(millis: f64) -> Option<u64> {
    (millis.is_finite() && millis > 0.).then(|| (millis / 1000.) as u64)
}

/// The site's facts about an app, as the catalog sorts, filters and shows them.
pub fn listing_of(site: &SiteApp) -> Listing {
    Listing {
        added_at: seconds(site.added_at),
        updated_at: site.last_release_at.and_then(seconds),
        ratings: Ratings { runs: site.recommended, issues: site.report_issues, broken: site.report_broken },
        project_type: site.project_type.trim().to_lowercase(),
        runs_on: site.supported_os.iter().map(|o| o.trim().to_lowercase()).collect(),
        ai: match site.ai_level {
            AiLevel::None => AiUse::None,
            AiLevel::Assisted => AiUse::Assisted,
            AiLevel::Generated => AiUse::Generated,
        },
        verified: site.verified_version().map(str::to_string),
        latest: site.last_release_version.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string),
        based_on: site.games.iter().map(|g| g.title.trim().to_string()).filter(|t| !t.is_empty()).collect(),
    }
}

/// The system the game came from: the site's first console when it names one Reclaw knows, else what the entry says.
pub fn system_of(entry: &AppEntry, site: Option<&SiteApp>) -> Platform {
    site.and_then(|s| s.consoles.iter().find_map(|c| Platform::from_tag(c))).unwrap_or_else(|| entry.system())
}

/// The release the screens call current: the one the site verified (what Reclaw installs), else the one the platform metadata
/// looked at.
fn current_release(app: &CatalogApp) -> Option<&str> {
    app.site.as_deref().and_then(SiteApp::verified_version).or_else(|| app.release.as_ref().map(|r| r.release_tag.as_str()))
}

/// A catalog app as the Catalog and the game page show it. The release is the verified one (or the one the platform metadata looked
/// at): its tag, with a link to its page. (The notes are not known here; the game page reads the site's releases itself.)
pub fn project_from(app: &CatalogApp, id: u32) -> ProjectInfo {
    let entry = &app.entry;
    let site = app.site.as_deref();
    let repo = repo_ref(entry);
    let extension = entry.extension.clone().unwrap_or_default();
    let published = site.and_then(|s| s.verified.as_ref()).and_then(|v| v.released_at).filter(|ms| ms.is_finite() && *ms > 0.);
    let releases = current_release(app)
        .filter(|tag| !tag.trim().is_empty())
        .map(|tag| Release {
            tag: tag.to_string(),
            name: tag.to_string(),
            published: published.map(crate::community::day).unwrap_or_default(),
            notes: String::new(),
            url: if entry.is_manual() { String::new() } else { release_url(&repo, tag) },
            prerelease: site.and_then(|s| s.verified.as_ref()).is_some_and(|v| v.prerelease),
        })
        .into_iter()
        .collect();
    let summary = extension.summary.clone().or_else(|| site.map(|s| s.description.trim().to_string()).filter(|d| !d.is_empty()));
    ProjectInfo {
        id,
        title: entry.name.clone(),
        project: entry.project.clone().unwrap_or_default(),
        summary: summary.unwrap_or_default(),
        description: extension.description.clone().unwrap_or_default(),
        platform: system_of(entry, site),
        repo,
        // Pictures an author put in the `reclaw` block win; then the site's SteamGridDB art; then the catalog's icon.
        hero_url: extension.hero_url.clone().or_else(|| site.and_then(SiteApp::hero).map(str::to_string)),
        capsule_url: extension
            .capsule_url
            .clone()
            .or_else(|| site.and_then(SiteApp::capsule).map(str::to_string))
            .or_else(|| entry.icon_url.clone()),
        media: extension.media(),
        releases,
        requirements: extension.requirements.clone(),
        tags: entry.tags.clone(),
        capabilities: extension.capabilities.clone().unwrap_or_default(),
        listing: site.map(listing_of),
    }
}

/// What the installer knows about an app, by its key (`key_of`). An app with no entry here is not installed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum InstallState {
    /// A finished install of this version is on disk. `latest` is a newer release found by asking the host service, which the
    /// catalog's own list may not have yet.
    Installed { version: String, latest: Option<String> },
    /// An install or an update is running.
    Installing,
    /// The last attempt failed and nothing is installed.
    Failed,
}

pub type InstallStates = std::collections::HashMap<String, InstallState>;

/// An app the user has added, as the Library shows it. `release` is the latest version the catalog knows of; `state` is what is
/// on disk. An installed app shows the version it has, and "update ready" when the catalog knows a newer one; an app that is not
/// installed shows the version it would install.
pub fn game_from(entry: &AppEntry, release: Option<&str>, id: u32, state: Option<&InstallState>) -> GameEntry {
    game_with(entry, None, release, id, state)
}

/// [`game_from`], with the site's entry for the app when it has one (its art and its system).
pub fn game_with(entry: &AppEntry, site: Option<&SiteApp>, release: Option<&str>, id: u32, state: Option<&InstallState>) -> GameEntry {
    let extension = entry.extension.clone().unwrap_or_default();
    let (status, version) = match state {
        Some(InstallState::Installed { version, latest }) => {
            let newer = latest.as_deref().or(release).is_some_and(|latest| reclaw_games::version::is_newer(latest, version));
            (if newer { AppStatus::UpdateReady } else { AppStatus::Installed }, version.clone())
        }
        Some(InstallState::Installing) => (AppStatus::Installing, release.unwrap_or_default().to_string()),
        Some(InstallState::Failed) => (AppStatus::Failed, release.unwrap_or_default().to_string()),
        None => (AppStatus::Available, release.unwrap_or_default().to_string()),
    };
    GameEntry {
        id,
        title: Cow::Owned(entry.custom_display_name.clone().unwrap_or_else(|| entry.name.clone())),
        project: Cow::Owned(entry.project.clone().unwrap_or_default()),
        version: Cow::Owned(version),
        source: source_of(entry),
        status,
        tags: entry.tags.iter().cloned().map(Cow::Owned).collect(),
        platform: system_of(entry, site),
        art: Art {
            capsule: extension
                .capsule_url
                .or_else(|| site.and_then(SiteApp::capsule).map(str::to_string))
                .or_else(|| entry.icon_url.clone()),
            hero: extension.hero_url.or_else(|| site.and_then(SiteApp::hero).map(str::to_string)),
            repo: (!entry.is_manual()).then(|| repo_ref(entry)),
        },
        in_library: true,
        run: RunState::Idle,
    }
}

/// Everything the screens need from the catalog and the library.
pub fn load(catalog: &[CatalogApp], library: &[AppEntry], states: &InstallStates) -> Loaded {
    let ids = IdMap::for_keys(catalog.iter().map(|a| key_of(&a.entry)).chain(library.iter().map(key_of)));
    let id = |entry: &AppEntry| ids.get(&key_of(entry)).unwrap_or_default();
    let projects: Vec<ProjectInfo> = catalog.iter().map(|app| project_from(app, id(&app.entry))).collect();
    let by_key: std::collections::HashMap<String, &CatalogApp> = catalog.iter().map(|a| (key_of(&a.entry), a)).collect();
    let games = library
        .iter()
        .map(|entry| {
            let key = key_of(entry);
            let app = by_key.get(&key).copied();
            let release = app.and_then(|a| current_release(a));
            game_with(entry, app.and_then(|a| a.site.as_deref()), release, id(entry), states.get(&key))
        })
        .collect();
    Loaded { projects, games }
}

/// The platform of a game, for code that has only the id (kept here so the mapping has one home).
pub fn platform_of(games: &[GameEntry], id: u32) -> Platform {
    games.iter().find(|g| g.id == id).map_or(Platform::Other, |g| g.platform)
}

#[cfg(test)]
mod tests;
