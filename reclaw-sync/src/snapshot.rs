//! What the catalog looked like at one moment: the index, every list that could be read, the platform metadata, and what went
//! wrong along the way. Plain data, so the UI's mapping from it is a pure function.
use std::collections::HashMap;

use reclaw_catalog::{AppEntry, CatalogList, CommunityIndex, IndexSource, PlatformDocument, PlatformIndex, platform_index::PlatformEntry};
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

/// An app of the catalog with what is known about it from the platform metadata.
#[derive(Clone, PartialEq, Debug)]
pub struct CatalogApp {
    pub entry: AppEntry,
    /// The names of the lists it is in, the first being where it was taken from.
    pub lists: Vec<String>,
    /// The release the platform metadata looked at, if it covers this app.
    pub release: Option<PlatformEntry>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct CatalogSnapshot {
    pub index: CommunityIndex,
    pub index_origin: Origin,
    pub lists: Vec<ListSnapshot>,
    pub platform: Option<PlatformSnapshot>,
    pub problems: Vec<Problem>,
    /// Unix seconds when the oldest part was last confirmed.
    pub fetched_at: u64,
}

impl CatalogSnapshot {
    /// Every app of every list, once: an app in two lists keeps the place and entry of the first and notes the other.
    pub fn apps(&self) -> Vec<CatalogApp> {
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
                        let release = self
                            .platform
                            .as_ref()
                            .and_then(|p| p.index.get(entry.source, &entry.repository, entry.preferred_version.as_deref()))
                            .cloned();
                        apps.push(CatalogApp { entry: entry.clone(), lists: vec![list_name.clone()], release });
                    }
                }
            }
        }
        apps
    }

    /// Whether any part is a saved copy standing in for one that could not be fetched.
    pub fn has_stale_parts(&self) -> bool {
        self.index_origin == Origin::Stale
            || self.lists.iter().any(|l| l.origin == Origin::Stale)
            || self.platform.as_ref().is_some_and(|p| p.origin == Origin::Stale)
    }
}

#[cfg(test)]
mod tests;
