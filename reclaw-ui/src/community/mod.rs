//! What players and quiverlauncher.com say about a game: how it runs (ratings and reviews), who made it, whether AI wrote it, and
//! what the site checked about each release. Plain data and wording, no Freya: the host fills it in, a game page shows it.
//!
//! * this file: [`Community`] (the part of the store), [`PageState`] and [`link_all`], which says which site entry each game is
//! * `words.rs`: everything worded for the screen (Quiver 3.5's `BrowseText`, `BrowseDetailsViewModel` and `ReleaseWarnings`,
//!   ported): the rating line, a review line, a release line, the facts beside a game
//! * `tests.rs`
mod words;

use std::collections::BTreeMap;

use reclaw_catalog::{
    AppEntry,
    site::{Detail, HistoryRelease, Links, Review, SiteApp, link},
};
use reclaw_sync::CatalogApp;

pub use words::*;

use crate::catalog_data::{IdMap, key_of};

/// An app's page as the host read it. Each part is read on its own and fails on its own: reviews that could not be read still leave
/// the releases to show.
#[derive(Clone, PartialEq, Debug)]
pub struct PageData {
    /// `Ok(None)`: the site no longer lists the app.
    pub detail: Result<Option<Detail>, String>,
    pub reviews: Result<Vec<Review>, String>,
    pub releases: Result<Vec<HistoryRelease>, String>,
}

#[derive(Clone, PartialEq, Debug)]
pub enum PageState {
    Loading,
    Loaded(Box<PageData>),
}

/// The part of the store: each game's entry in the site's listing (by game id), and the pages read so far.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Community {
    pub apps: BTreeMap<u32, SiteApp>,
    pub pages: BTreeMap<u32, PageState>,
}

/// One game's share of it, for a page.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct CommunityOf {
    pub app: Option<SiteApp>,
    pub page: Option<PageState>,
}

impl Community {
    pub fn of(&self, id: u32) -> CommunityOf {
        CommunityOf { app: self.apps.get(&id).cloned(), page: self.pages.get(&id).cloned() }
    }
}

/// Each game's site entry, by the id the screens know it by. A library entry is linked by what it carries (an entry id Quiver
/// wrote), so it wins over the catalog's copy of the same game.
pub fn link_all(catalog: &[CatalogApp], library: &[AppEntry], links: &Links) -> BTreeMap<u32, SiteApp> {
    let ids = IdMap::for_keys(catalog.iter().map(|a| key_of(&a.entry)).chain(library.iter().map(key_of)));
    let mut out = BTreeMap::new();
    for entry in catalog.iter().map(|a| &a.entry).chain(library) {
        let (Some(id), Some(app)) = (ids.get(&key_of(entry)), link(entry, links).and_then(|slug| links.app(&slug))) else { continue };
        out.insert(id, app.clone());
    }
    out
}

#[cfg(test)]
mod tests;
