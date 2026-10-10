//! Asking quiverlauncher.com, through the shared client (ported from Quiver 3.5's `QuiverCatalogClient`): the whole listing and the
//! release status feed (to link apps and show what players said on cards and pages), and one app's page, reviews and release
//! history when its page opens. Every call blocks; call from a worker thread.
//!
//! The listing and the feed are saved and used offline. An app's own page is asked fresh each time it opens, with a short life on
//! disk so going back and forth does not ask again.
//!
//! When `api.quiverlauncher.com` cannot be reached at all (no answer, a failed TLS handshake, its proxy's 499), the deployment's own
//! host is asked instead and, once it answers, for the rest of the run. The error shown is the first one: it is the player's real
//! problem.
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use reclaw_catalog::site::{self, Detail, HistoryRelease, Links, Page, ReadError, ReleaseStatus, Review, SiteApp};
use reclaw_net::{Fetched, Net, NetError, Request};
use serde::de::DeserializeOwned;

/// The listing and the feed change when an app is added or a release is checked: Quiver reads them every 30 minutes.
pub const LISTING_TTL: Duration = Duration::from_secs(30 * 60);
/// An app's page, reviews and releases: fresh enough to go back and forth, short enough that a new review shows on the next visit.
pub const PAGE_TTL: Duration = Duration::from_secs(2 * 60);
const MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Far more pages than the catalog has (it lists a few hundred apps); a cursor that never ends stops here.
const MAX_PAGES: usize = 50;
const TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SiteError {
    #[error("quiverlauncher.com could not be reached: {0}")]
    Net(NetError),
    #[error("{0}")]
    Read(ReadError),
    #[error("quiverlauncher.com kept giving more pages after {MAX_PAGES}; the rest were not read")]
    Endless,
}

impl SiteError {
    pub fn hint(&self) -> Option<&'static str> {
        match self {
            Self::Net(e) => e.hint(),
            _ => None,
        }
    }
}

#[derive(Clone)]
pub struct SiteClient {
    net: Net,
    base: String,
    /// The deployment's own host, for when the custom domain cannot be reached. Only with the real API.
    fallback: Option<String>,
    on_fallback: Arc<AtomicBool>,
}

impl SiteClient {
    /// The real API, with its fallback host.
    pub fn new(net: Net) -> Self {
        Self { net, base: site::API.to_string(), fallback: Some(site::FALLBACK_API.to_string()), on_fallback: Arc::default() }
    }

    /// Another address (a test's own server, or `QUIVER_API`), with no fallback.
    pub fn with_base(net: Net, base: impl Into<String>) -> Self {
        Self { net, base: base.into().trim_end_matches('/').to_string(), fallback: None, on_fallback: Arc::default() }
    }

    /// A test's two hosts: the first plays the custom domain, the second the deployment's host.
    #[cfg(test)]
    fn with_fallback(net: Net, base: &str, fallback: &str) -> Self {
        Self { fallback: Some(fallback.to_string()), ..Self::with_base(net, base) }
    }

    /// The custom domain could not be reached at all, so the fallback may answer. A status means it did answer.
    fn unreachable(error: &NetError) -> bool {
        matches!(
            error,
            NetError::Dns { .. }
                | NetError::Connect { .. }
                | NetError::Tls { .. }
                | NetError::Timeout
                | NetError::Stalled(_)
                | NetError::Status { status: 499, .. }
        )
    }

    fn request(base: &str, path: &str, ttl: Duration) -> Request {
        Request::get(format!("{base}{path}")).accept("application/json").max_bytes(MAX_BYTES).timeout(TIMEOUT).cached(ttl, true)
    }

    fn get(&self, path: &str, ttl: Duration) -> Result<Fetched, NetError> {
        if let Some(fallback) = self.fallback.as_deref().filter(|_| self.on_fallback.load(Ordering::Relaxed)) {
            return self.net.fetch(&Self::request(fallback, path, ttl));
        }
        let first = match self.net.fetch(&Self::request(&self.base, path, ttl)) {
            Ok(answer) => return Ok(answer),
            Err(error) => error,
        };
        let Some(fallback) = self.fallback.as_deref().filter(|_| Self::unreachable(&first)) else { return Err(first) };
        match self.net.fetch(&Self::request(fallback, path, ttl)) {
            Ok(answer) => {
                tracing::warn!(error = %first, fallback, "quiverlauncher.com's API could not be reached; using its deployment host from now on");
                self.on_fallback.store(true, Ordering::Relaxed);
                Ok(answer)
            }
            Err(second) => {
                tracing::debug!(error = %second, "the fallback host did not answer either");
                Err(first)
            }
        }
    }

    fn all<T: DeserializeOwned>(&self, path: impl Fn(Option<&str>) -> String) -> Result<Vec<T>, SiteError> {
        let (mut items, mut cursor, mut skipped) = (Vec::new(), None::<String>, 0);
        for _ in 0..MAX_PAGES {
            let fetched = self.get(&path(cursor.as_deref()), LISTING_TTL).map_err(SiteError::Net)?;
            let page: Page<T> = site::parse_page(&fetched.text()).map_err(SiteError::Read)?;
            skipped += page.skipped;
            items.extend(page.items);
            cursor = page.next_cursor;
            if cursor.is_none() {
                if skipped > 0 {
                    tracing::warn!(skipped, path = %path(None), "quiverlauncher.com entries that could not be read were left out");
                }
                return Ok(items);
            }
        }
        Err(SiteError::Endless)
    }

    /// Every app, and the feed that links each to its repository. Either failing fails both: a listing without the feed links
    /// nothing, and a feed without the listing would link apps to entries with nothing to show.
    pub fn links(&self) -> Result<Links, SiteError> {
        let status: Vec<ReleaseStatus> = self.all(site::path::release_status)?;
        let apps: Vec<SiteApp> = self.all(site::path::apps)?;
        tracing::info!(apps = apps.len(), linked = status.len(), "quiverlauncher.com catalog read");
        Ok(Links::new(status, apps))
    }

    /// An app's page; `None` when the site no longer lists it.
    pub fn detail(&self, slug: &str) -> Result<Option<Detail>, SiteError> {
        match self.get(&site::path::detail(slug), PAGE_TTL) {
            Ok(fetched) => site::parse_one(&fetched.text()).map_err(SiteError::Read),
            Err(NetError::Status { status: 404, .. }) => Ok(None),
            Err(error) => Err(SiteError::Net(error)),
        }
    }

    /// The newest reviews, up to [`site::REVIEWS_SHOWN`].
    pub fn reviews(&self, slug: &str) -> Result<Vec<Review>, SiteError> {
        let fetched = self.get(&site::path::reviews(slug, site::REVIEWS_SHOWN), PAGE_TTL).map_err(SiteError::Net)?;
        Ok(site::parse_page(&fetched.text()).map_err(SiteError::Read)?.items)
    }

    /// The app's releases as the site judges them, newest first. A site that does not list them yet (404) has none.
    pub fn release_history(&self, slug: &str) -> Result<Vec<HistoryRelease>, SiteError> {
        match self.get(&site::path::release_history(slug), PAGE_TTL) {
            Ok(fetched) => Ok(site::parse_page(&fetched.text()).map_err(SiteError::Read)?.items),
            Err(NetError::Status { status: 404, .. }) => Ok(Vec::new()),
            Err(error) => Err(SiteError::Net(error)),
        }
    }
}

#[cfg(test)]
mod tests;
