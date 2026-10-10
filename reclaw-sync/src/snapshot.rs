//! What the catalog looked like at one moment: quiverlauncher.com's listing, the community index and every list that could be read,
//! the platform metadata, and what went wrong along the way. Plain data, so the UI's mapping from it is a pure function.
//!
//! Since Quiver 3.5 the site is the catalog and the lists are frozen (ADR 0025): the site's apps come first, newest first, and a list
//! entry the site also has adds only what the site lacks (Reclaw's own `reclaw` block, the platform metadata's release). A list
//! entry the site does not have still shows, after them, so nothing the lists offered disappears.
use std::collections::HashMap;

use reclaw_catalog::{
    AppEntry, CatalogList, CommunityIndex, IndexSource, PlatformDocument, PlatformIndex,
    platform_index::PlatformEntry,
    site::{self, Links, SiteApp},
};
use reclaw_net::Source;

/// Where a piece of the snapshot came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    Network,
    /// A saved copy the server confirmed is current.
    Revalidated,
    /// A saved copy young enough to need no question.
    Saved,
    /// A saved copy used because asking failed.
    Stale,
}

impl From<Source> for Origin {
    fn from(source: Source) -> Self {
        match source {
            Source::Network => Self::Network,
            Source::CacheRevalidated => Self::Revalidated,
            Source::CacheFresh => Self::Saved,
            Source::CacheStale => Self::Stale,
        }
    }
}

/// Something that went wrong with one part, in words for the person, with what to do about it when there is something.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Problem {
    pub what: String,
    pub detail: String,
    pub hint: Option<String>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ListSnapshot {
    pub source: IndexSource,
    pub list: CatalogList,
    pub origin: Origin,
    pub fetched_at: u64,
}

#[derive(Clone, PartialEq, Debug)]
pub struct PlatformSnapshot {
    pub document: PlatformDocument,
    pub index: PlatformIndex,
    pub origin: Origin,
    pub fetched_at: u64,
}

/// quiverlauncher.com's listing as it was read.
#[derive(Clone, PartialEq, Debug)]
pub struct SiteSnapshot {
    pub links: Links,
    pub origin: Origin,
    pub fetched_at: u64,
}

/// What the catalog's list of where an app came from calls the site.
pub const SITE_LIST: &str = "quiverlauncher.com";

/// An app of the catalog with what is known about it from the site and the platform metadata.
#[derive(Clone, PartialEq, Debug)]
pub struct CatalogApp {
    pub entry: AppEntry,
    /// The names of the lists it is in, the first being where it was taken from ([`SITE_LIST`] for the site).
    pub lists: Vec<String>,
    /// The release the platform metadata looked at, if it covers this app.
    pub release: Option<PlatformEntry>,
    /// The site's entry for it: its ratings, art, dates and verified release. `None` for an app only the frozen lists have.
    pub site: Option<Box<SiteApp>>,
}

impl CatalogApp {
    pub fn from_list(entry: AppEntry, list: &str) -> Self {
        Self { entry, lists: vec![list.to_string()], release: None, site: None }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CatalogSnapshot {
    pub index: CommunityIndex,
    pub index_origin: Origin,
    pub lists: Vec<ListSnapshot>,
    pub platform: Option<PlatformSnapshot>,
    /// The site's listing; `None` when it could not be read and nothing was saved (the lists are all there is then).
    pub site: Option<SiteSnapshot>,
    pub problems: Vec<Problem>,
    /// Unix seconds when the oldest part was last confirmed.
    pub fetched_at: u64,
}

impl CatalogSnapshot {
    /// Every app the catalog offers, once: the site's, newest first, then the lists' entries the site does not have. With no
    /// library to align to (see [`apps_for`](Self::apps_for)).
    pub fn apps(&self) -> Vec<CatalogApp> {
        self.apps_for(&[])
    }

    /// [`apps`](Self::apps), with each site app the library already holds (linked as Quiver links it: its entry id, its repository,
    /// its folder) taking the library's repository and folder, so the two are one game to the screens rather than a library tile
    /// and a catalog card that look alike.
    pub fn apps_for(&self, library: &[AppEntry]) -> Vec<CatalogApp> {
        let from_lists = self.list_apps();
        let Some(site) = &self.site else { return from_lists };
        let mut apps: Vec<CatalogApp> = Vec::new();
        let (mut by_slug, mut by_key) = (HashMap::new(), HashMap::new());
        let mut listed: Vec<&SiteApp> = site.links.listed().collect();
        // Newest first, as Quiver opens its catalog ("Recently added"); the listing's own order breaks a tie.
        listed.sort_by(|a, b| b.added_at.total_cmp(&a.added_at));
        let mut owned: HashMap<String, &AppEntry> = HashMap::new();
        for own in library {
            if let Some(slug) = site::link(own, &site.links) {
                owned.entry(slug.to_lowercase()).or_insert(own);
            }
        }
        let mut unmapped = 0usize;
        for app in listed {
            let Some(mut entry) = site::to_entry(app, site.links.status_of(app)) else {
                unmapped += 1;
                continue;
            };
            if let Some(own) = owned.get(&app.slug.to_lowercase()) {
                entry.repository = own.repository.clone();
                entry.source = own.source;
                entry.folder_name = own.folder_name.clone();
            }
            let key = entry.instance_key().to_uppercase();
            if by_key.contains_key(&key) {
                continue;
            }
            let release = self.release_of(&entry);
            by_slug.insert(app.slug.to_lowercase(), apps.len());
            by_key.insert(key, apps.len());
            apps.push(CatalogApp { entry, lists: vec![SITE_LIST.to_string()], release, site: Some(Box::new(app.clone())) });
        }
        if unmapped > 0 {
            tracing::debug!(unmapped, "quiverlauncher.com apps with no line in the release feed were left out");
        }
        for list_app in from_lists {
            let linked = site::link(&list_app.entry, &site.links).and_then(|slug| by_slug.get(&slug.to_lowercase()).copied());
            match linked.or_else(|| by_key.get(&list_app.entry.instance_key().to_uppercase()).copied()) {
                Some(at) => {
                    let into = &mut apps[at];
                    into.entry.extension = into.entry.extension.take().or(list_app.entry.extension);
                    into.entry.catalog_id = into.entry.catalog_id.take().or(list_app.entry.catalog_id);
                    into.release = into.release.take().or(list_app.release);
                    into.lists.extend(list_app.lists);
                }
                None => {
                    by_key.insert(list_app.entry.instance_key().to_uppercase(), apps.len());
                    apps.push(list_app);
                }
            }
        }
        apps
    }

    fn release_of(&self, entry: &AppEntry) -> Option<PlatformEntry> {
        self.platform.as_ref().and_then(|p| p.index.get(entry.source, &entry.repository, entry.preferred_version.as_deref())).cloned()
    }

    /// Every app of every list, once: an app in two lists keeps the place and entry of the first and notes the other.
    fn list_apps(&self) -> Vec<CatalogApp> {
        let mut apps: Vec<CatalogApp> = Vec::new();
        let mut place: HashMap<String, usize> = HashMap::new();
        for snapshot in &self.lists {
            let list_name = snapshot.list.name.clone().filter(|n| !n.is_empty()).unwrap_or_else(|| snapshot.source.name.clone());
            for entry in &snapshot.list.apps {
                let key = entry.instance_key().to_uppercase();
                match place.get(&key) {
                    Some(&at) => apps[at].lists.push(list_name.clone()),
                    None => {
                        place.insert(key, apps.len());
                        let release = self.release_of(entry);
                        apps.push(CatalogApp { entry: entry.clone(), lists: vec![list_name.clone()], release, site: None });
                    }
                }
            }
        }
        apps
    }

    /// Whether any part is a saved copy standing in for one that could not be fetched.
    pub fn has_stale_parts(&self) -> bool {
        self.index_origin == Origin::Stale
            || self.site.as_ref().is_some_and(|s| s.origin == Origin::Stale)
            || self.lists.iter().any(|l| l.origin == Origin::Stale)
            || self.platform.as_ref().is_some_and(|p| p.origin == Origin::Stale)
    }
}

#[cfg(test)]
mod tests;
