//! Browsing the Catalog the way Quiver 3.5 and quiverlauncher.com do: sorted by when an app was added or last released, by what
//! players said, or by name; narrowed to the apps that run here, to one kind of project, or away from AI-written ones; and with the
//! apps already in the library left out so new ones are easier to find. Pure: the settings say how, the projects carry the site's
//! facts (`ProjectInfo::listing`), and the result is what the page draws.
//!
//! Quiver asks the site to sort and filter, a page at a time. Reclaw has the whole listing already (it is the catalog), so it does
//! both here, instantly and offline. A project only the frozen lists have carries no facts: it sorts after the site's apps and is
//! kept by a filter that cannot judge it, except one that asks for a kind of project (nobody said it is one).
use std::collections::HashMap;

use reclaw_games::{
    listing::{AiUse, Listing},
    project::{Platform, ProjectInfo},
};

use crate::{
    catalog::matches_query,
    community::{Rating, Tone},
    model::GameEntry,
    settings::{SettingsTarget, SettingsValues},
};

pub const KEY_CATALOG_SORT: &str = "catalog_sort";
pub const KEY_CATALOG_RUNS_ON: &str = "catalog_runs_on";
pub const KEY_CATALOG_KIND: &str = "catalog_kind";
pub const KEY_CATALOG_AI: &str = "catalog_ai";
pub const KEY_CATALOG_HIDE_LIBRARY: &str = "catalog_hide_library";

/// In the order of [`CatalogSort::ALL`]: the saved choice is a position.
pub const SORT_OPTIONS: &[&str] = &["Recently added", "Recently updated", "Top rated", "Name A-Z", "System"];
pub const RUNS_ON_OPTIONS: &[&str] = &["Runs on this computer", "All platforms"];
pub const KIND_OPTIONS: &[&str] = &["All project types", "Port", "Tool", "Emulator", "Standalone game"];
pub const AI_OPTIONS: &[&str] = &["Show all apps", "Hide mostly AI-generated apps", "Hide apps with any AI use"];

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CatalogSort {
    /// Newest on the site first: how the catalog opens.
    #[default]
    Added,
    /// Newest release first.
    Updated,
    TopRated,
    Name,
    /// Grouped by console, oldest first, A to Z within.
    System,
}

impl CatalogSort {
    pub const ALL: [Self; 5] = [Self::Added, Self::Updated, Self::TopRated, Self::Name, Self::System];
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AiFilter {
    #[default]
    All,
    NoGenerated,
    NoAi,
}

/// How the person wants the Catalog shown, from the Catalog section of Settings.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CatalogView {
    pub sort: CatalogSort,
    /// Only apps that say they run on this operating system.
    pub this_system_only: bool,
    /// `port`, `tool`, `emulator` or `game`; `None` for every kind.
    pub kind: Option<&'static str>,
    pub ai: AiFilter,
    pub hide_library: bool,
}

impl Default for CatalogView {
    fn default() -> Self {
        Self { sort: CatalogSort::Added, this_system_only: true, kind: None, ai: AiFilter::All, hide_library: true }
    }
}

impl CatalogView {
    pub fn from_settings(values: &SettingsValues) -> Self {
        let choice = |key, default| values.choice(SettingsTarget::Global, key, default);
        Self {
            sort: CatalogSort::ALL.get(choice(KEY_CATALOG_SORT, 0)).copied().unwrap_or_default(),
            this_system_only: choice(KEY_CATALOG_RUNS_ON, 0) == 0,
            kind: [None, Some("port"), Some("tool"), Some("emulator"), Some("game")].get(choice(KEY_CATALOG_KIND, 0)).copied().flatten(),
            ai: [AiFilter::All, AiFilter::NoGenerated, AiFilter::NoAi].get(choice(KEY_CATALOG_AI, 0)).copied().unwrap_or_default(),
            hide_library: values.toggle(SettingsTarget::Global, KEY_CATALOG_HIDE_LIBRARY, true),
        }
    }
}

/// The site's id for the operating system Reclaw was built for.
pub fn this_os() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "android") {
        "android"
    } else {
        "linux"
    }
}

fn keeps(listing: Option<&Listing>, view: &CatalogView, os: &str) -> bool {
    let Some(listing) = listing else { return view.kind.is_none() };
    let runs_here = !view.this_system_only || listing.runs_on.is_empty() || listing.runs_on.iter().any(|o| o == os);
    let kind = view.kind.is_none_or(|k| listing.project_type == k);
    let ai = match view.ai {
        AiFilter::All => true,
        AiFilter::NoGenerated => listing.ai != AiUse::Generated,
        AiFilter::NoAi => listing.ai == AiUse::None,
    };
    runs_here && kind && ai
}

/// What the Catalog shows.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Browse {
    pub entries: Vec<GameEntry>,
    /// Apps that matched but were left out for being in the library.
    pub hidden_in_library: usize,
}

/// The projects that pass the console chip, the search and the view's filters, as library entries when the person has added them,
/// in the view's order. `games` is the library.
pub fn browse(games: &[GameEntry], projects: &[ProjectInfo], system: Option<Platform>, query: &str, view: &CatalogView) -> Browse {
    let os = this_os();
    let mut matching: Vec<(&ProjectInfo, GameEntry)> = projects
        .iter()
        .filter(|p| system.is_none_or(|wanted| p.platform == wanted))
        .filter(|p| keeps(p.listing.as_ref(), view, os))
        .map(|p| (p, games.iter().find(|g| g.id == p.id).cloned().unwrap_or_else(|| GameEntry::from_project(p))))
        .filter(|(_, g)| matches_query(g, query))
        .collect();
    let before = matching.len();
    if view.hide_library {
        matching.retain(|(_, g)| !g.in_library);
    }
    let hidden_in_library = before - matching.len();
    let title = |g: &GameEntry| g.title.to_lowercase();
    // `None` sorts last: `Reverse(None) < Reverse(Some)` would put it first, so the key is (missing, newest first).
    let newest = |at: Option<u64>| (at.is_none(), std::cmp::Reverse(at.unwrap_or(0)));
    match view.sort {
        // The host lists the site's apps newest first and the lists' own after them: keep that.
        CatalogSort::Added => {}
        CatalogSort::Updated => matching.sort_by_cached_key(|(p, g)| (newest(p.listing.as_ref().and_then(|l| l.updated_at)), title(g))),
        CatalogSort::TopRated => matching.sort_by_cached_key(|(p, g)| {
            // A project only the lists have has no ratings at all: it goes last, after the site's unrated ones.
            let rank = p.listing.as_ref().map(|l| l.ratings.rank()).unwrap_or_default();
            (std::cmp::Reverse(rank), title(g))
        }),
        CatalogSort::Name => matching.sort_by_cached_key(|(_, g)| title(g)),
        CatalogSort::System => matching.sort_by_cached_key(|(_, g)| (g.platform.rank(), title(g))),
    }
    Browse { entries: matching.into_iter().map(|(_, g)| g).collect(), hidden_in_library }
}

/// What a catalog card adds under its title: how it runs, and whether it is new.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CardNote {
    /// "Runs well · 4 players".
    pub rating: String,
    pub tone: Tone,
    pub new: bool,
}

/// The note for each project the site lists, by id.
pub fn card_notes(projects: &[ProjectInfo], now: u64) -> HashMap<u32, CardNote> {
    projects
        .iter()
        .filter_map(|p| {
            let listing = p.listing.as_ref()?;
            let rating = Rating::new(listing.ratings.runs, listing.ratings.issues, listing.ratings.broken);
            Some((p.id, CardNote { rating: rating.line(), tone: rating.tone, new: listing.is_new(now) }))
        })
        .collect()
}

#[cfg(test)]
mod tests;
