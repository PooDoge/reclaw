//! From what the catalog says and what the user keeps, to what the screens show: `ProjectInfo`s for the Catalog and the game page,
//! `GameEntry`s for the Library. Pure, so the mapping is tested on plain data and against the real catalog.
//!
//! The screens identify a game by a `u32` that is saved (favorites, per-game settings), so it must be the same on every start and
//! must not depend on the order the catalog lists things: [`IdMap`] derives it from the app's identity.
use std::{borrow::Cow, collections::BTreeMap};

use reclaw_catalog::AppEntry;
use reclaw_games::project::{Platform, ProjectInfo, Release, RepoHost, RepoRef};
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

/// A catalog app as the Catalog and the game page show it. The release is the one the platform metadata looked at: its tag, with a
/// link to its page. (The date and notes are not in that file; the game page fetches the README itself.)
pub fn project_from(app: &CatalogApp, id: u32) -> ProjectInfo {
    let entry = &app.entry;
    let repo = repo_ref(entry);
    let extension = entry.extension.clone().unwrap_or_default();
    let releases = app
        .release
        .as_ref()
        .filter(|r| !r.release_tag.is_empty())
        .map(|r| Release {
            tag: r.release_tag.clone(),
            name: r.release_tag.clone(),
            published: String::new(),
            notes: String::new(),
            url: release_url(&repo, &r.release_tag),
            prerelease: false,
        })
        .into_iter()
        .collect();
    ProjectInfo {
        id,
        title: entry.name.clone(),
        project: entry.project.clone().unwrap_or_default(),
        summary: extension.summary.clone().unwrap_or_default(),
        description: extension.description.clone().unwrap_or_default(),
        platform: entry.system(),
        repo,
        hero_url: extension.hero_url.clone(),
        // The catalog gives an icon, which is the best picture it has; a portrait capsule from the `reclaw` block wins over it.
        capsule_url: extension.capsule_url.clone().or_else(|| entry.icon_url.clone()),
        media: extension.media(),
        releases,
        requirements: extension.requirements.clone(),
        tags: entry.tags.clone(),
        capabilities: extension.capabilities.clone().unwrap_or_default(),
    }
}

/// An app the user has added, as the Library shows it. Whether it is installed is not known here (the installer owns that), so it
/// is "not installed" with the latest known version.
pub fn game_from(entry: &AppEntry, release: Option<&str>, id: u32) -> GameEntry {
    let extension = entry.extension.clone().unwrap_or_default();
    GameEntry {
        id,
        title: Cow::Owned(entry.custom_display_name.clone().unwrap_or_else(|| entry.name.clone())),
        project: Cow::Owned(entry.project.clone().unwrap_or_default()),
        version: Cow::Owned(release.unwrap_or_default().to_string()),
        source: source_of(entry),
        status: AppStatus::Available,
        tags: entry.tags.iter().cloned().map(Cow::Owned).collect(),
        platform: entry.system(),
        art: Art { capsule: extension.capsule_url.or_else(|| entry.icon_url.clone()), hero: extension.hero_url },
        in_library: true,
        run: RunState::Idle,
    }
}

/// Everything the screens need from the catalog and the library.
pub fn load(catalog: &[CatalogApp], library: &[AppEntry]) -> Loaded {
    let ids = IdMap::for_keys(catalog.iter().map(|a| key_of(&a.entry)).chain(library.iter().map(key_of)));
    let id = |entry: &AppEntry| ids.get(&key_of(entry)).unwrap_or_default();
    let projects: Vec<ProjectInfo> = catalog.iter().map(|app| project_from(app, id(&app.entry))).collect();
    let games = library
        .iter()
        .map(|entry| {
            let release =
                catalog.iter().find(|a| key_of(&a.entry) == key_of(entry)).and_then(|a| a.release.as_ref()).map(|r| r.release_tag.as_str());
            game_from(entry, release, id(entry))
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
