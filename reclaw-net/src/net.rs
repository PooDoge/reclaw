//! [`Net`]: the one HTTP client. It owns a small runtime so the rest of the program can stay synchronous (a worker thread
//! calls `fetch` and waits), two `reqwest` clients (one that unpacks compressed answers, one that never touches the bytes
//! of a download), the per-host politeness, and the disk cache.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use reqwest::{
    Client, Response,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE, ETAG, HeaderMap, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED},
    redirect::Policy,
};
use tokio::runtime::{Handle, Runtime};
use url::Url;

use crate::{
    cache::{Entry, FileCache, Meta, unix_now},
    challenge::is_bot_challenge,
    config::{NetConfig, ProxyMode},
    error::{NetError, from_reqwest},
    gate::HostGate,
    limits::limited_for,
    request::{Fetched, Request, Source},
    retry,
};

/// A rate-limit wait up to this long is slept through; a longer one is reported and remembered.
const SHORT_WAIT: Duration = Duration::from_secs(5);
/// How much of an error response is read to recognise a limit or a challenge in it.
const ERROR_BODY_PEEK: usize = 4096;

pub(crate) struct Inner {
    pub(crate) cfg: NetConfig,
    pub(crate) api: Client,
    pub(crate) files: Client,
    pub(crate) gate: HostGate,
    pub(crate) cache: Option<FileCache>,
    runtime: Mutex<Option<Runtime>>,
    handle: Handle,
}

impl Drop for Inner {
    fn drop(&mut self) {
        // Dropping a runtime inside another one panics; `shutdown_background` is allowed anywhere.
        let runtime = self.runtime.get_mut().map(Option::take).unwrap_or_else(|e| e.into_inner().take());
        if let Some(runtime) = runtime {
            runtime.shutdown_background();
        }
    }
}

/// The network. Cheap to clone; every clone shares the connections, the cache and the limits.
#[derive(Clone)]
pub struct Net {
    pub(crate) inner: Arc<Inner>,
}

impl PartialEq for Net {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl std::fmt::Debug for Net {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Net").field("config", &self.inner.cfg).finish()
    }
}

fn build_client(cfg: &NetConfig, decode: bool, stall: Option<Duration>) -> Result<Client, NetError> {
    // Every hop of a redirect goes through the same address rules as the first request.
    let policy = cfg.address_policy;
    let max = cfg.max_redirects;
    let redirects = Policy::custom(move |attempt| {
        if attempt.previous().len() >= max {
            attempt.error("too many redirects")
        } else if let Err(why) = policy.check(attempt.url()) {
            attempt.error(why)
        } else {
            attempt.follow()
        }
    });
    let mut builder = Client::builder()
        .user_agent(cfg.user_agent.clone())
        .connect_timeout(cfg.connect_timeout)
        .redirect(redirects)
        .pool_idle_timeout(Duration::from_secs(60))
        .tcp_keepalive(Duration::from_secs(30))
        .tcp_nodelay(true)
        .http2_adaptive_window(true)
        .http2_keep_alive_interval(Duration::from_secs(20))
        .http2_keep_alive_while_idle(true)
        .http2_keep_alive_timeout(Duration::from_secs(10));
    if let Some(stall) = stall {
        builder = builder.read_timeout(stall);
    }
    if !decode {
        // A release archive must arrive byte for byte, even from a server that labels it `Content-Encoding: gzip`.
        builder = builder.no_gzip().no_brotli().no_deflate();
    }
    builder = match &cfg.proxy {
        ProxyMode::System => builder,
        ProxyMode::None => builder.no_proxy(),
        ProxyMode::Url(url) => {
            let proxy = reqwest::Proxy::all(url).map_err(|e| NetError::Other(format!("the proxy address {url} is not valid: {e}")))?;
            builder.proxy(proxy.no_proxy(reqwest::NoProxy::from_env()))
        }
    };
    if let Some(pem) = &cfg.extra_roots_pem {
        let roots = reqwest::Certificate::from_pem_bundle(pem)
            .map_err(|e| NetError::Other(format!("the extra certificates are not valid PEM: {e}")))?;
        for root in roots {
            builder = builder.add_root_certificate(root);
        }
    }
    builder.build().map_err(|e| NetError::Other(format!("could not set up the HTTP client: {e}")))
}

/// What one attempt at a request came to when it did not succeed.
pub(crate) enum Attempt {
    /// Try again, after `wait` if the server said how long.
    Retry {
        error: NetError,
        wait: Option<Duration>,
    },
    Stop(NetError),
}

enum Outcome {
    Body(Fetched),
    NotModified,
}

impl Net {
    pub fn new(cfg: NetConfig) -> Result<Self, NetError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(cfg.worker_threads.max(1))
            .thread_name("reclaw-net")
            .enable_all()
            .build()
            .map_err(|e| NetError::Other(format!("could not start the network runtime: {e}")))?;
        let api = build_client(&cfg, true, None)?;
        let files = build_client(&cfg, false, Some(cfg.stall_timeout))?;
        let cache = cfg.cache_dir.clone().map(FileCache::new);
        let gate = HostGate::new(cfg.per_host_concurrency);
        let handle = runtime.handle().clone();
        Ok(Self { inner: Arc::new(Inner { cfg, api, files, gate, cache, runtime: Mutex::new(Some(runtime)), handle }) })
    }

    pub fn config(&self) -> &NetConfig {
        &self.inner.cfg
    }

    pub fn cache(&self) -> Option<&FileCache> {
        self.inner.cache.as_ref()
    }

    /// Run a future on the network's runtime and wait. Errors instead of panicking when called from async code.
    pub(crate) fn block<T>(&self, future: impl std::future::Future<Output = T>) -> Result<T, NetError> {
        if Handle::try_current().is_ok() {
            return Err(NetError::WrongContext);
        }
        Ok(self.inner.handle.block_on(future))
    }

    /// Fetch a small answer, using the disk cache as the request says. Call from a worker thread, not from async code.
    pub fn fetch(&self, request: &Request) -> Result<Fetched, NetError> {
        self.block(self.fetch_async(request))?
    }

    /// The saved copy of `request`, of any age, without touching the network. `None` when nothing was saved (or caching is off
    /// or the request has no cache rule). What an offline start has to show.
    pub fn cached(&self, request: &Request) -> Option<Fetched> {
        let url = self.parse(&request.url).ok()?;
        let cache = self.inner.cache.as_ref()?;
        request.cache?;
        let credential = self.inner.cfg.credential_tag(url.host_str().unwrap_or_default());
        let entry = cache.read(url.as_str(), &credential)?;
        let fresh = request.cache.is_some_and(|rule| entry.age_secs(unix_now()) < rule.ttl.as_secs());
        Some(from_entry(&entry, if fresh { Source::CacheFresh } else { Source::CacheStale }, None))
    }

    pub(crate) fn parse(&self, text: &str) -> Result<Url, NetError> {
        self.inner.cfg.address_policy.parse(text).map_err(NetError::Blocked)
    }

    pub(crate) fn auth_header(&self, url: &Url) -> Option<HeaderValue> {
        let token = self.inner.cfg.token_for(url.host_str()?)?;
        let mut value = HeaderValue::from_str(&format!("Bearer {token}")).ok()?;
        value.set_sensitive(true);
        (url.scheme() == "https" || self.inner.cfg.address_policy == crate::address::AddressPolicy::AnyHttpForTests).then_some(value)
    }

    async fn fetch_async(&self, request: &Request) -> Result<Fetched, NetError> {
        let url = self.parse(&request.url)?;
        let host = url.host_str().unwrap_or_default().to_string();
        let credential = self.inner.cfg.credential_tag(&host);
        let now = unix_now();
        let saved = match (&self.inner.cache, request.cache) {
            (Some(cache), Some(_)) => cache.read(url.as_str(), &credential),
            _ => None,
        };
        if let (Some(rule), Some(entry)) = (request.cache, &saved)
            && entry.age_secs(now) < rule.ttl.as_secs()
        {
            return Ok(from_entry(entry, Source::CacheFresh, None));
        }
        let validators = saved.as_ref().map(|e| &e.meta);
        match self.network(request, &url, validators).await {
            Ok(Outcome::Body(fetched)) => {
                if let (Some(cache), Some(_)) = (&self.inner.cache, request.cache) {
                    let entry = Entry {
                        meta: Meta {
                            url: url.to_string(),
                            etag: fetched.etag.clone(),
                            last_modified: fetched.last_modified.clone(),
                            content_type: fetched.content_type.clone(),
                            fetched_at: fetched.fetched_at,
                        },
                        body: fetched.body.clone(),
                    };
                    // A cache that cannot be written costs speed, not correctness; the answer is still good.
                    if let Err(e) = cache.write(url.as_str(), &credential, &entry) {
                        eprintln!("reclaw-net: could not save {url} to the cache: {e}");
                    }
                }
                Ok(fetched)
            }
            Ok(Outcome::NotModified) => match saved {
                Some(mut entry) => {
                    entry.meta.fetched_at = unix_now();
                    if let Some(cache) = &self.inner.cache
                        && let Err(e) = cache.write(url.as_str(), &credential, &entry)
                    {
                        eprintln!("reclaw-net: could not refresh {url} in the cache: {e}");
                    }
                    Ok(from_entry(&entry, Source::CacheRevalidated, None))
                }
                None => Err(NetError::Other(format!("{host} answered 304 to a request that had no validators"))),
            },
            Err(error) => match (request.cache, saved) {
                (Some(rule), Some(entry)) if rule.stale_on_error && error.is_connectivity() => {
                    Ok(from_entry(&entry, Source::CacheStale, Some(error)))
                }
                _ => Err(error),
            },
        }
    }

    async fn network(&self, request: &Request, url: &Url, validators: Option<&Meta>) -> Result<Outcome, NetError> {
        let host = url.host_str().unwrap_or_default().to_string();
        let attempts = self.inner.cfg.attempts.max(1);
        for attempt in 0..attempts {
            let permit = self.inner.gate.enter(&host).await?;
            let result = self.once(request, url, validators).await;
            drop(permit);
            match result {
                Ok(outcome) => return Ok(outcome),
                Err(Attempt::Retry { error, wait }) if attempt + 1 < attempts => {
                    let wait = wait.unwrap_or_else(|| retry::delay(attempt, retry::jitter()));
                    tokio::time::sleep(wait).await;
                    drop(error);
                }
                Err(Attempt::Retry { error, .. } | Attempt::Stop(error)) => return Err(error),
            }
        }
        Err(NetError::Other("no attempt was made".into()))
    }

    async fn once(&self, request: &Request, url: &Url, validators: Option<&Meta>) -> Result<Outcome, Attempt> {
        let host = url.host_str().unwrap_or_default().to_string();
        let mut builder = self.inner.api.get(url.clone()).timeout(request.timeout);
        if let Some(accept) = &request.accept {
            builder = builder.header(ACCEPT, accept);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some(auth) = self.auth_header(url) {
            builder = builder.header(AUTHORIZATION, auth);
        }
        if let Some(meta) = validators {
            if let Some(etag) = &meta.etag {
                builder = builder.header(IF_NONE_MATCH, etag);
            }
            if let Some(modified) = &meta.last_modified {
                builder = builder.header(IF_MODIFIED_SINCE, modified);
            }
        }
        let response = builder.send().await.map_err(|e| self.transport(&host, &e))?;
        let status = response.status().as_u16();
        if status == 304 {
            return Ok(Outcome::NotModified);
        }
        if !response.status().is_success() {
            return Err(self.rejected(&host, response).await);
        }
        if response.content_length().is_some_and(|len| len > request.max_bytes) {
            return Err(Attempt::Stop(NetError::TooLarge { limit: request.max_bytes }));
        }
        let headers = response.headers().clone();
        let final_url = response.url().to_string();
        let body = read_body(response, request.max_bytes).await.map_err(|e| match e {
            BodyError::TooLarge => Attempt::Stop(NetError::TooLarge { limit: request.max_bytes }),
            BodyError::Net(e) => self.transport(&host, &e),
        })?;
        let text = |name| headers.get(name).and_then(|v: &HeaderValue| v.to_str().ok()).map(str::to_string);
        Ok(Outcome::Body(Fetched {
            url: final_url,
            status,
            body,
            content_type: text(CONTENT_TYPE),
            etag: text(ETAG),
            last_modified: text(LAST_MODIFIED),
            fetched_at: unix_now(),
            source: Source::Network,
            stale_because: None,
        }))
    }

    /// A transport failure: classified, and retried when it says nothing about the request.
    pub(crate) fn transport(&self, host: &str, error: &reqwest::Error) -> Attempt {
        let error = from_reqwest(host, error);
        if error.is_transient() { Attempt::Retry { error, wait: None } } else { Attempt::Stop(error) }
    }

    /// A non-success answer: a limit, a bot check, a server fault or a plain refusal.
    pub(crate) async fn rejected(&self, host: &str, response: Response) -> Attempt {
        let status = response.status().as_u16();
        let headers: HeaderMap = response.headers().clone();
        let peek = peek_body(response, ERROR_BODY_PEEK).await;
        let text = String::from_utf8_lossy(&peek).into_owned();
        if is_bot_challenge(status, &headers, &peek) {
            return Attempt::Stop(NetError::BotChallenge { host: host.to_string(), status });
        }
        if let Some(wait) = limited_for(status, &headers, &text, std::time::SystemTime::now()) {
            let error = NetError::RateLimited { host: host.to_string(), retry_in: wait };
            if wait <= SHORT_WAIT {
                return Attempt::Retry { error, wait: Some(wait) };
            }
            self.inner.gate.block(host, wait);
            return Attempt::Stop(error);
        }
        let error = NetError::Status { host: host.to_string(), status };
        if error.is_transient() {
            Attempt::Retry { error, wait: crate::limits::retry_after(&headers, std::time::SystemTime::now()) }
        } else {
            Attempt::Stop(error)
        }
    }
}

fn from_entry(entry: &Entry, source: Source, stale_because: Option<NetError>) -> Fetched {
    Fetched {
        url: entry.meta.url.clone(),
        status: 200,
        body: entry.body.clone(),
        content_type: entry.meta.content_type.clone(),
        etag: entry.meta.etag.clone(),
        last_modified: entry.meta.last_modified.clone(),
        fetched_at: entry.meta.fetched_at,
        source,
        stale_because,
    }
}

enum BodyError {
    TooLarge,
    Net(reqwest::Error),
}

async fn read_body(mut response: Response, max: u64) -> Result<Vec<u8>, BodyError> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(BodyError::Net)? {
        if body.len() as u64 + chunk.len() as u64 > max {
            return Err(BodyError::TooLarge);
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

/// Up to `limit` bytes of a body, stopping early; a failure to read just yields what arrived.
async fn peek_body(mut response: Response, limit: usize) -> Vec<u8> {
    let mut body = Vec::new();
    while body.len() < limit {
        match response.chunk().await {
            Ok(Some(chunk)) => body.extend_from_slice(&chunk[..chunk.len().min(limit - body.len())]),
            Ok(None) | Err(_) => break,
        }
    }
    body
}
