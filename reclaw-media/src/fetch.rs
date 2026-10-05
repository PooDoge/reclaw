//! Getting bytes from the network. The [`Fetch`] trait is the seam: the cache and the hub work with
//! any implementation, and their tests use a fake one. The real one is a thin layer over the program's shared
//! HTTP client (`reclaw-net`), which brings the user agent, the proxy and certificate settings, retries and the
//! address policy.
use std::time::Duration;

use reclaw_net::{Net, NetError, Request};

use crate::source::{MediaUrl, UrlError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fetched {
    pub bytes: Vec<u8>,
    /// What the server said it was. Informational only; see `sniff`.
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FetchError {
    #[error("the server answered {0}")]
    Status(u16),
    #[error("larger than the {limit} byte limit")]
    TooLarge { limit: u64 },
    #[error("timed out")]
    Timeout,
    #[error("{0}")]
    Network(String),
    #[error("a redirect led somewhere not allowed: {0}")]
    Blocked(UrlError),
}

pub trait Fetch: Send + Sync {
    /// The body of `url`, which must be at most `max_bytes`.
    fn get(&self, url: &MediaUrl, max_bytes: u64) -> Result<Fetched, FetchError>;
}

/// The real network, through the shared client.
pub struct HttpFetcher {
    net: Net,
    timeout: Duration,
}

impl HttpFetcher {
    /// `timeout` is the time allowed for one picture or document, body included.
    pub fn new(net: Net, timeout: Duration) -> Self {
        Self { net, timeout }
    }
}

impl Fetch for HttpFetcher {
    fn get(&self, url: &MediaUrl, max_bytes: u64) -> Result<Fetched, FetchError> {
        let request = Request::get(url.as_str()).max_bytes(max_bytes).timeout(self.timeout);
        // The network layer has already logged what went wrong; here it is only the picture that did not come.
        let fetched = self.net.fetch(&request).map_err(|error| {
            tracing::debug!(url = %url.as_str().split('?').next().unwrap_or_default(), %error, "artwork or document not fetched");
            classify(error)
        })?;
        Ok(Fetched { bytes: fetched.body, content_type: fetched.content_type })
    }
}

/// What the media cache remembers about a failure is coarser than what the network layer knows.
fn classify(error: NetError) -> FetchError {
    match error {
        NetError::Status { status, .. } => FetchError::Status(status),
        NetError::TooLarge { limit } => FetchError::TooLarge { limit },
        NetError::Timeout | NetError::Stalled(_) => FetchError::Timeout,
        NetError::Blocked(why) => FetchError::Blocked(why),
        other => FetchError::Network(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_failures_keep_the_distinctions_the_cache_acts_on() {
        let host = || "h".to_string();
        assert_eq!(classify(NetError::Status { host: host(), status: 404 }), FetchError::Status(404));
        assert_eq!(classify(NetError::TooLarge { limit: 5 }), FetchError::TooLarge { limit: 5 });
        assert_eq!(classify(NetError::Timeout), FetchError::Timeout);
        assert_eq!(classify(NetError::Blocked(UrlError::LocalNetwork)), FetchError::Blocked(UrlError::LocalNetwork));
        assert!(matches!(classify(NetError::ProxyDenied { host: host(), status: None }), FetchError::Network(m) if m.contains("refused")));
    }
}
