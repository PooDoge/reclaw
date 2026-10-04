//! Getting bytes from the network. The [`Fetch`] trait is the seam: the cache and the hub work with
//! any implementation, and their tests use a fake one.
use std::{io::Read, sync::OnceLock, time::Duration};

use reqwest::{blocking::Client, redirect::Policy};

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

/// The real network. The HTTP client is made on first use, on the worker thread that uses it: the
/// blocking client must not be created or dropped inside an async runtime, and the UI thread may be in one.
pub struct HttpFetcher {
    timeout: Duration,
    client: OnceLock<Result<Client, String>>,
}

const MAX_REDIRECTS: usize = 4;

impl HttpFetcher {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout, client: OnceLock::new() }
    }

    fn client(&self) -> Result<&Client, FetchError> {
        let built = self.client.get_or_init(|| {
            // Every hop of a redirect goes through the same address rules as the first request.
            let redirects = Policy::custom(|attempt| {
                if attempt.previous().len() >= MAX_REDIRECTS {
                    attempt.error("too many redirects")
                } else if let Err(why) = MediaUrl::check(attempt.url()) {
                    attempt.error(why)
                } else {
                    attempt.follow()
                }
            });
            Client::builder()
                .user_agent(concat!("Reclaw/", env!("CARGO_PKG_VERSION")))
                .timeout(self.timeout)
                .connect_timeout(Duration::from_secs(8))
                .redirect(redirects)
                .build()
                .map_err(|e| e.to_string())
        });
        built.as_ref().map_err(|e| FetchError::Network(e.clone()))
    }
}

impl Fetch for HttpFetcher {
    fn get(&self, url: &MediaUrl, max_bytes: u64) -> Result<Fetched, FetchError> {
        let response = self.client()?.get(url.url().clone()).send().map_err(classify)?;
        let status = response.status();
        if !status.is_success() {
            return Err(FetchError::Status(status.as_u16()));
        }
        // Refuse a body that announces it is too big before reading any of it.
        if response.content_length().is_some_and(|len| len > max_bytes) {
            return Err(FetchError::TooLarge { limit: max_bytes });
        }
        let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).map(str::to_string);
        // A body may lie about its length or have none (chunked), so the read is capped too.
        let mut bytes = Vec::new();
        response.take(max_bytes + 1).read_to_end(&mut bytes).map_err(|e| FetchError::Network(e.to_string()))?;
        if bytes.len() as u64 > max_bytes {
            return Err(FetchError::TooLarge { limit: max_bytes });
        }
        Ok(Fetched { bytes, content_type })
    }
}

fn classify(error: reqwest::Error) -> FetchError {
    if error.is_timeout() {
        FetchError::Timeout
    } else {
        // The redirect policy's refusal arrives wrapped as a network error; keep its words.
        FetchError::Network(error.to_string())
    }
}
