//! Which quiverlauncher.com entry an app is, ported from Quiver 3.5's `CatalogReleases.Find`. The site's entries do not carry the
//! community lists' keys, so an app is matched by what it has, most certain first:
//!
//! 1. its `catalogEntryId` (a library app Quiver added from the site, or linked before), which survives renames;
//! 2. its `catalogId` from the community lists, which the site kept on the entries it took over (Reclaw's addition: Quiver 3.5 does
//!    not look at it, but it is what links a list entry exactly);
//! 3. its provider and repository; when two entries share a repository (one release with several games), the one with the same
//!    download filter, else the one with the same folder;
//! 4. no entry has its repository (it moved since it was added): the one entry with its folder.
//!
//! Still two candidates: no link. Better unlinked than shown another app's reviews.
use std::collections::HashMap;

use super::{ReleaseStatus, SiteApp};
use crate::entry::AppEntry;

/// What the site said, indexed for linking: the release status feed (repositories) and the listing (folders, filters, everything
/// shown), by slug.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Links {
    pub status: Vec<ReleaseStatus>,
    pub apps: HashMap<String, SiteApp>,
    /// The slugs (lower case) in the order the listing gave them, each once.
    order: Vec<String>,
}

impl Links {
    pub fn new(status: Vec<ReleaseStatus>, apps: Vec<SiteApp>) -> Self {
        let mut order = Vec::with_capacity(apps.len());
        let mut by_slug = HashMap::with_capacity(apps.len());
        for app in apps {
            let slug = app.slug.to_lowercase();
            if by_slug.insert(slug.clone(), app).is_none() {
                order.push(slug);
            }
        }
        Self { status, apps: by_slug, order }
    }

    /// Every app, in the listing's order.
    pub fn listed(&self) -> impl Iterator<Item = &SiteApp> {
        self.order.iter().filter_map(|slug| self.apps.get(slug))
    }

    /// The app's line of the release status feed: by its id, else by its slug.
    pub fn status_of(&self, app: &SiteApp) -> Option<&ReleaseStatus> {
        let id = app.id.trim();
        self.status
            .iter()
            .find(|s| !id.is_empty() && s.id == id)
            .or_else(|| self.status.iter().find(|s| s.slug.eq_ignore_ascii_case(app.slug.trim())))
    }

    pub fn app(&self, slug: &str) -> Option<&SiteApp> {
        self.apps.get(&slug.to_lowercase())
    }

    /// The folder the site gives an app: its own, or its slug when it gives none (Quiver's `FolderFor`).
    fn folder(&self, slug: &str) -> Option<String> {
        let app = self.app(slug)?;
        Some(super::folder_for(app))
    }

    fn filter(&self, slug: &str) -> String {
        self.app(slug).and_then(|a| crate::normalize::asset_filter(a.launcher.release_asset_filter.as_deref())).unwrap_or_default()
    }
}

fn same(a: &str, b: &str) -> bool {
    a.trim().eq_ignore_ascii_case(b.trim())
}

fn only<T>(mut found: Vec<T>) -> Option<T> {
    if found.len() == 1 { found.pop() } else { None }
}

/// The slug of the app's entry on the site, if it has exactly one.
pub fn link(entry: &AppEntry, links: &Links) -> Option<String> {
    // A manual app is not downloaded from anywhere; the site's entries all are.
    if entry.is_manual() {
        return None;
    }
    if let Some(id) = entry.catalog_entry_id.as_deref().filter(|id| !id.trim().is_empty())
        && let Some(found) = links.status.iter().find(|s| s.id == id.trim())
    {
        return Some(found.slug.clone());
    }
    if let Some(id) = entry.catalog_id.as_deref().filter(|id| !id.trim().is_empty())
        && let Some(found) = only(links.apps.values().filter(|a| a.catalog_id.as_deref().is_some_and(|c| same(c, id))).collect())
    {
        return Some(found.slug.clone());
    }
    let provider = entry.source.as_str();
    let ours = |s: &&ReleaseStatus| same(&s.provider, provider);
    let same_folder = |s: &&ReleaseStatus| links.folder(&s.slug).is_some_and(|f| same(&f, &entry.folder_name));
    let candidates: Vec<&ReleaseStatus> =
        links.status.iter().filter(ours).filter(|s| s.repository.as_deref().is_some_and(|r| same(r, &entry.repository))).collect();
    let found = match candidates.len() {
        0 => only(links.status.iter().filter(ours).filter(same_folder).collect()),
        1 => candidates.first().copied(),
        _ => {
            let filter = entry.release_asset_filter.clone().unwrap_or_default();
            only(candidates.iter().copied().filter(|c| same(&links.filter(&c.slug), &filter)).collect())
                .or_else(|| only(candidates.iter().copied().filter(same_folder).collect()))
        }
    };
    found.map(|s| s.slug.clone())
}
