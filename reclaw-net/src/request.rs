//! What to ask for and what comes back, for the small answers (a catalog file, a release list). Large files use
//! `download`.
use std::time::Duration;

use crate::error::NetError;

/// Keep an answer on disk and reuse it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CacheRule {
    /// A copy younger than this is used without asking the server.
    pub ttl: Duration,
    /// When asking fails because the network is down or unwell, hand back the old copy instead of an error.
    pub stale_on_error: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Request {
    pub url: String,
    pub accept: Option<String>,
    /// Extra headers (`X-GitHub-Api-Version`, say). Not for credentials: those come from the configuration.
    pub headers: Vec<(String, String)>,
    /// The largest body accepted, decoded. Checked as it arrives, not only from `Content-Length`.
    pub max_bytes: u64,
    /// Time allowed for the whole exchange, body included.
    pub timeout: Duration,
    pub cache: Option<CacheRule>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            accept: None,
            headers: Vec::new(),
            max_bytes: 8 * 1024 * 1024,
            timeout: Duration::from_secs(30),
            cache: None,
        }
    }

    pub fn accept(mut self, accept: &str) -> Self {
        self.accept = Some(accept.to_string());
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn max_bytes(mut self, max: u64) -> Self {
        self.max_bytes = max;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn cached(mut self, ttl: Duration, stale_on_error: bool) -> Self {
        self.cache = Some(CacheRule { ttl, stale_on_error });
        self
    }
}

/// Where an answer came from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Source {
    /// Fetched just now.
    Network,
    /// A saved copy young enough to need no question.
    CacheFresh,
    /// A saved copy the server confirmed is still current (`304`).
    CacheRevalidated,
    /// A saved copy used because asking failed; see `Fetched::stale_because`.
    CacheStale,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Fetched {
    /// The address after redirects.
    pub url: String,
    pub status: u16,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    /// Unix seconds when the server last vouched for these bytes.
    pub fetched_at: u64,
    pub source: Source,
    pub stale_because: Option<NetError>,
}

impl Fetched {
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    pub fn is_stale(&self) -> bool {
        self.source == Source::CacheStale
    }
}
