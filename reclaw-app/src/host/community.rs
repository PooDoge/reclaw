//! What quiverlauncher.com says about the games. The site's listing and release feed are read with the catalog (they are the
//! catalog now, ADR 0025); after every refresh each game is linked to its entry (`community::link_all`), which gives the cards and
//! pages their ratings and facts, and library apps from the site follow its name, project, icon and tags (`site::follow`). A game's
//! own page (who made it, reviews, what the site checked about each release) is read when the page opens.
//!
//! A page that cannot be read is not a notice: it says so on the page, and the failure is logged.
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, MutexGuard},
    thread,
};

use reclaw_catalog::site::{self, Links};
use reclaw_net::Net;
use reclaw_sync::{SiteClient, SiteError};
use reclaw_ui::{
    community::{self, PageData, PageState},
    store::AppAction,
};

use crate::host::Host;

/// What an install needs to know about a game's entry on the site.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Linked {
    pub slug: String,
    /// The release the site verified, from its status feed.
    pub verified: Option<String>,
}

/// What the host keeps for the site.
pub struct Community {
    client: Option<SiteClient>,
    /// Each game's entry on the site, by game id, from the last linking.
    linked: Mutex<HashMap<u32, Linked>>,
    /// Pages being read, so opening a page twice asks once.
    loading: Mutex<HashSet<u32>>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn words(error: &SiteError) -> String {
    match error.hint() {
        Some(hint) => format!("{error}. {hint}"),
        None => error.to_string(),
    }
}

impl Community {
    /// `api` is another address than the real one (a test's server, or `QUIVER_API`).
    pub fn new(net: Option<&Net>, api: Option<String>) -> Self {
        let client = net.map(|net| match api {
            Some(api) => SiteClient::with_base(net.clone(), api),
            None => SiteClient::new(net.clone()),
        });
        Self { client, linked: Mutex::default(), loading: Mutex::default() }
    }

    /// The client, shared with the catalog's refresh so both use the fallback host once one has needed it.
    pub fn client(&self) -> Option<SiteClient> {
        self.client.clone()
    }

    /// A game's entry on the site, from the last linking.
    pub fn linked(&self, id: u32) -> Option<Linked> {
        lock(&self.linked).get(&id).cloned()
    }
}

impl Host {
    /// Link every game to the site's listing (read with the catalog) and tell the screens what the site says about each.
    pub(crate) fn publish_community(&self, links: &Links) {
        let apps = {
            let state = self.state();
            community::link_all(&state.catalog, &state.library, links)
        };
        tracing::info!(linked = apps.len(), "games linked to quiverlauncher.com");
        *lock(&self.inner.community.linked) = apps
            .iter()
            .map(|(id, app)| {
                // The status feed is what Quiver trusts for the verified release; the listing's copy is the fallback.
                let fed = links.status_of(app).and_then(|s| s.verified.as_ref()).map(|v| v.version.trim()).filter(|v| !v.is_empty());
                (*id, Linked { slug: app.slug.clone(), verified: fed.or_else(|| app.verified_version()).map(str::to_string) })
            })
            .collect();
        self.send(AppAction::SetCommunity(apps));
    }

    /// Library apps from the site take its current name, project, icon and tags, unless the person changed that field since the site
    /// last set it; saved once if anything moved. A library that cannot be written is left alone (it was reported when it was read).
    pub(crate) fn follow_site(&self, links: &Links) {
        let mut state = self.state();
        if !state.library_writable {
            return;
        }
        let mut next = state.library.clone();
        let mut moved = 0usize;
        for entry in next.iter_mut() {
            let Some(app) = site::link(entry, links).and_then(|slug| links.app(&slug)) else { continue };
            if site::follow(entry, app) {
                moved += 1;
            }
        }
        if moved == 0 {
            return;
        }
        match self.inner.store.save(&next) {
            Ok(()) => {
                tracing::info!(apps = moved, "library apps took quiverlauncher.com's current name, icon or tags");
                state.library = next;
            }
            Err(error) => {
                drop(state);
                tracing::warn!(%error, "the library could not be saved after following quiverlauncher.com");
            }
        }
    }

    /// A game's page opened: read its page, reviews and releases from the site, unless it has no entry there or is being read.
    pub(crate) fn load_community_page(&self, id: u32) {
        let community = &self.inner.community;
        let (Some(client), Some(slug)) = (community.client.clone(), lock(&community.linked).get(&id).map(|l| l.slug.clone())) else {
            return;
        };
        if !lock(&community.loading).insert(id) {
            return;
        }
        self.send(AppAction::CommunityPage { id, page: PageState::Loading });
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-site-page".into()).spawn(move || {
            let page = read_page(&client, &slug);
            lock(&host.inner.community.loading).remove(&id);
            host.send(AppAction::CommunityPage { id, page: PageState::Loaded(Box::new(page)) });
        });
        if let Err(error) = started {
            lock(&community.loading).remove(&id);
            let failed = format!("a background thread did not start: {error}");
            let page = PageData { detail: Err(failed.clone()), reviews: Err(failed.clone()), releases: Err(failed) };
            self.send(AppAction::CommunityPage { id, page: PageState::Loaded(Box::new(page)) });
        }
    }
}

/// The three parts of a page, each failing on its own. A failure is logged with the slug and shown on the page in words.
fn read_page(client: &SiteClient, slug: &str) -> PageData {
    let note = |part: &str, error: &SiteError| {
        tracing::warn!(slug, part, error = %error, "part of a game's quiverlauncher.com page could not be read");
        words(error)
    };
    PageData {
        detail: client.detail(slug).map_err(|e| note("page", &e)),
        reviews: client.reviews(slug).map_err(|e| note("reviews", &e)),
        releases: client.release_history(slug).map_err(|e| note("releases", &e)),
    }
}

#[cfg(test)]
mod tests;
