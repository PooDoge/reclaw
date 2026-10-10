//! What quiverlauncher.com says about the games. After every catalog refresh the site's listing and release feed are read in the
//! background and each game is linked to its entry (`community::link_all`), which gives the cards and pages their ratings and
//! facts. A game's own page (who made it, reviews, what the site checked about each release) is read when the page opens.
//!
//! Nothing here is shown as a notice: the site is extra information. A failure is logged, and a page says it could not be read.
use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, MutexGuard, atomic::AtomicBool, atomic::Ordering},
    thread,
};

use reclaw_catalog::site::Links;
use reclaw_net::Net;
use reclaw_sync::{SiteClient, SiteError};
use reclaw_ui::{
    community::{self, PageData, PageState},
    store::AppAction,
};

use crate::host::Host;

/// What the host keeps for the site.
pub struct Community {
    client: Option<SiteClient>,
    /// Each game's slug, by game id, from the last linking.
    slugs: Mutex<HashMap<u32, String>>,
    /// Pages being read, so opening a page twice asks once.
    loading: Mutex<HashSet<u32>>,
    reading: AtomicBool,
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
        Self { client, slugs: Mutex::default(), loading: Mutex::default(), reading: AtomicBool::new(false) }
    }
}

impl Host {
    /// Read the site's listing and link every game to it, in the background. One at a time; a second request while one runs does
    /// nothing (the catalog refresh that asked for it is the only caller).
    pub(crate) fn refresh_community(&self) {
        let community = &self.inner.community;
        let Some(client) = community.client.clone() else { return };
        if community.reading.swap(true, Ordering::SeqCst) {
            return;
        }
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-site".into()).spawn(move || {
            host.run_community_refresh(&client);
            host.inner.community.reading.store(false, Ordering::SeqCst);
        });
        if let Err(error) = started {
            community.reading.store(false, Ordering::SeqCst);
            tracing::error!(%error, "the quiverlauncher.com listing could not be read: a thread did not start");
        }
    }

    /// The body of [`refresh_community`](Self::refresh_community); runs directly in tests.
    pub(crate) fn run_community_refresh(&self, client: &SiteClient) {
        match client.links() {
            Ok(links) => self.publish_community(&links),
            Err(error) => tracing::warn!(error = %error, hint = error.hint(), "quiverlauncher.com's listing could not be read"),
        }
    }

    fn publish_community(&self, links: &Links) {
        let apps = {
            let state = self.state();
            community::link_all(&state.catalog, &state.library, links)
        };
        tracing::info!(linked = apps.len(), "games linked to quiverlauncher.com");
        *lock(&self.inner.community.slugs) = apps.iter().map(|(id, app)| (*id, app.slug.clone())).collect();
        self.send(AppAction::SetCommunity(apps));
    }

    /// A game's page opened: read its page, reviews and releases from the site, unless it has no entry there or is being read.
    pub(crate) fn load_community_page(&self, id: u32) {
        let community = &self.inner.community;
        let (Some(client), Some(slug)) = (community.client.clone(), lock(&community.slugs).get(&id).cloned()) else { return };
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
