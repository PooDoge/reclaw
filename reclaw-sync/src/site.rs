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
use reclaw_net::{Fetched, Net, NetError, Request, Source};
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

/// Every page of one list, and whether any page was a saved copy standing in for a failed request.
struct Read<T> {
    items: Vec<T>,
    stale: Option<NetError>,
    fetched_at: u64,
    /// Where the oldest page came from.
    source: Source,
}

/// The whole catalog as the site lists it, and how fresh that is.
#[derive(Clone, PartialEq, Debug)]
pub struct Listing {
    pub links: Links,
    /// Why a saved copy was used instead of a fresh answer, when one was.
    pub stale: Option<NetError>,
    /// Unix seconds when the oldest page was last confirmed.
    pub fetched_at: u64,
    /// Where the oldest page came from (the network, or a saved copy).
    pub source: Source,
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
        // A saved copy stands in for an unreachable host, but the fallback may have a fresh answer: ask it first.
        let (first, saved) = match self.net.fetch(&Self::request(&self.base, path, ttl)) {
            Ok(answer) => match answer.stale_because.clone() {
                Some(reason) if self.fallback.is_some() && Self::unreachable(&reason) => (reason, Some(answer)),
                _ => return Ok(answer),
            },
            Err(error) => (error, None),
        };
        let Some(fallback) = self.fallback.as_deref().filter(|_| Self::unreachable(&first)) else { return saved.ok_or(first) };
        match self.net.fetch(&Self::request(fallback, path, ttl)).ok().filter(|a| saved.is_none() || a.stale_because.is_none()) {
            Some(answer) => {
                tracing::warn!(error = %first, fallback, "quiverlauncher.com's API could not be reached; using its deployment host from now on");
                self.on_fallback.store(true, Ordering::Relaxed);
                Ok(answer)
            }
            None => {
                tracing::debug!(fallback, "the fallback host did not answer either");
                saved.ok_or(first)
            }
        }
    }

    /// The saved copy of a request, from whichever host answered it last; nothing touches the network.
    fn saved(&self, path: &str) -> Option<Fetched> {
        let primary = self.net.cached(&Self::request(&self.base, path, LISTING_TTL));
        primary.or_else(|| self.fallback.as_deref().and_then(|f| self.net.cached(&Self::request(f, path, LISTING_TTL))))
    }

    fn all<T: DeserializeOwned>(&self, path: impl Fn(Option<&str>) -> String, saved_only: bool) -> Result<Read<T>, SiteError> {
        let (mut items, mut cursor, mut skipped) = (Vec::new(), None::<String>, 0);
        let (mut stale, mut fetched_at, mut source) = (None::<NetError>, u64::MAX, Source::Network);
        for _ in 0..MAX_PAGES {
            let fetched = if saved_only {
                self.saved(&path(cursor.as_deref())).ok_or_else(|| SiteError::Net(NetError::Other("nothing was saved".into())))?
            } else {
                self.get(&path(cursor.as_deref()), LISTING_TTL).map_err(SiteError::Net)?
            };
            if let Some(reason) = &fetched.stale_because {
                stale = stale.or_else(|| Some(reason.clone()));
            }
            if fetched.fetched_at <= fetched_at {
                source = fetched.source;
            }
            fetched_at = fetched_at.min(fetched.fetched_at);
            let page: Page<T> = site::parse_page(&fetched.text()).map_err(SiteError::Read)?;
            skipped += page.skipped;
            items.extend(page.items);
            cursor = page.next_cursor;
            if cursor.is_none() {
                if skipped > 0 {
                    tracing::warn!(skipped, path = %path(None), "quiverlauncher.com entries that could not be read were left out");
                }
                return Ok(Read { items, stale, fetched_at, source });
            }
        }
        Err(SiteError::Endless)
    }

    fn listing_with(&self, saved_only: bool) -> Result<Listing, SiteError> {
        let status: Read<ReleaseStatus> = self.all(site::path::release_status, saved_only)?;
        let apps: Read<SiteApp> = self.all(site::path::apps, saved_only)?;
        if !saved_only {
            tracing::info!(apps = apps.items.len(), linked = status.items.len(), "quiverlauncher.com catalog read");
        }
        Ok(Listing {
            source: if status.fetched_at <= apps.fetched_at { status.source } else { apps.source },
            stale: status.stale.or(apps.stale),
            fetched_at: status.fetched_at.min(apps.fetched_at),
            links: Links::new(status.items, apps.items),
        })
    }

    /// Every app and the feed that links each to its repository, through the cache (a saved copy stands in when the site cannot be
    /// reached). Either failing fails both: a listing without the feed has no repositories, and a feed without the listing has
    /// nothing to show.
    pub fn listing(&self) -> Result<Listing, SiteError> {
        self.listing_with(false)
    }

    /// What was saved by an earlier run, with no network at all; `None` when nothing (or only part) was saved.
    pub fn saved_listing(&self) -> Option<Listing> {
        self.listing_with(true).ok()
    }

    /// The listing's links alone.
    pub fn links(&self) -> Result<Links, SiteError> {
        self.listing().map(|l| l.links)
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

/// What went wrong with a download of a release's file, as the site's report takes it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DownloadProblem {
    /// The file is gone (a 404 or 410): its developer deleted it since the site listed it.
    Missing,
    /// The file is not the one the site checked (its SHA-256 differs).
    Mismatch,
}

impl DownloadProblem {
    fn word(self) -> &'static str {
        match self {
            Self::Missing => "missing",
            Self::Mismatch => "mismatch",
        }
    }
}

impl SiteClient {
    /// Tell the site a download of one of an app's files failed, as Quiver 3.5 does: it reads that release back from its repository
    /// straight away and takes it down or pulls it if it should; the report changes nothing by itself. Only a nudge: a failure is
    /// returned for the log, never shown.
    pub fn report_download_problem(&self, slug: &str, version: &str, file: &str, problem: DownloadProblem) -> Result<(), SiteError> {
        let base = match self.fallback.as_deref() {
            Some(fallback) if self.on_fallback.load(Ordering::Relaxed) => fallback,
            _ => self.base.as_str(),
        };
        let body = serde_json::json!({ "version": version, "file": file, "problem": problem.word() }).to_string().into_bytes();
        let request = Request::post_json(format!("{base}{}", site::path::download_problem(slug)), body)
            .accept("application/json")
            .max_bytes(64 * 1024)
            .timeout(TIMEOUT);
        self.net.fetch(&request).map(|_| ()).map_err(SiteError::Net)
    }
}

#[cfg(test)]
mod tests;
