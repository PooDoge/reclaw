//! [`Net`]: the one HTTP client. It owns a small runtime so the rest of the program can stay synchronous (a worker thread
//! calls `fetch` and waits), two `reqwest` clients (one that unpacks compressed answers, one that never touches the bytes
//! of a download), the per-host politeness, and the disk cache.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use reclaw_log::Secret;
use reqwest::{
    Client, Response,
    header::{ACCEPT, CONTENT_TYPE, ETAG, HeaderMap, HeaderName, HeaderValue, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED},
    redirect::Policy,
};
use tokio::runtime::{Handle, Runtime};
use url::Url;

use crate::{
    address::describe,
    cache::{Entry, FileCache, Meta, unix_now},
    challenge::is_bot_challenge,
    config::{NetConfig, ProxyMode},
    credentials::{Credentials, Provider, TokenState},
    error::{NetError, from_reqwest},
    gate::HostGate,
    limits::limited_for,
    request::{Fetched, Request, Source},
    retry,
};

/// A rate-limit wait up to this long is slept through; a longer one is reported and remembered.
const SHORT_WAIT: Duration = Duration::from_secs(5);
/// How long a refusal by the network's policy is remembered per host.
const POLICY_MEMORY: Duration = Duration::from_secs(5 * 60);
/// How much of an error response is read to recognise a limit or a challenge in it.
const ERROR_BODY_PEEK: usize = 4096;

/// Told, once per token, that a service refused it.
type RejectionHook = Arc<dyn Fn(&str) + Send + Sync>;

pub(crate) struct Inner {
    pub(crate) cfg: NetConfig,
    pub(crate) api: Client,
    pub(crate) files: Client,
    pub(crate) gate: HostGate,
    pub(crate) cache: Option<FileCache>,
    pub(crate) creds: Credentials,
    /// One lock per cached address and identity: a second asker for the same thing waits for the first and reads what it saved.
    flights: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    on_rejected: Mutex<Option<RejectionHook>>,
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

/// Holding the lock for one cached answer. Dropping it lets the next asker in and tidies the map when nobody else is waiting.
struct Flight {
    map: Arc<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>>,
    key: String,
    lock: Arc<tokio::sync::Mutex<()>>,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl Drop for Flight {
    fn drop(&mut self) {
        let mut map = self.map.lock().unwrap_or_else(|e| e.into_inner());
        // The map holds one reference, `lock` another and the guard a third: anything more is a waiter, which needs the entry.
        if Arc::strong_count(&self.lock) <= 3 {
            map.remove(&self.key);
        }
    }
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
        let creds = Credentials::new(&cfg.tokens);
        Ok(Self {
            inner: Arc::new(Inner {
                cfg,
                api,
                files,
                gate,
                cache,
                creds,
                flights: Arc::default(),
                on_rejected: Mutex::new(None),
                runtime: Mutex::new(Some(runtime)),
                handle,
            }),
        })
    }

    /// Use `token` for `provider` from now on (or none, with `None`). A wait recorded while asking anonymously is forgotten: the
    /// new identity has its own allowance. Answers already cached stay with the identity that fetched them.
    pub fn set_token(&self, provider: Provider, token: Option<Secret>) {
        self.set_host_token(provider.host(), token);
    }

    /// [`set_token`](Self::set_token) for an arbitrary host: for tests against a server on this machine.
    pub fn set_host_token(&self, host: &str, token: Option<Secret>) {
        let had = self.inner.creds.state(host) != TokenState::None;
        self.inner.creds.set(host, token);
        self.inner.gate.clear(host);
        let has = self.inner.creds.state(host) != TokenState::None;
        // Setting up at start (a token from the environment, or none) is routine; a token appearing or going is news.
        if had == has && !has {
            tracing::debug!(%host, "no access token for this host");
        } else if had == has {
            tracing::debug!(%host, "the access token for this host was replaced");
        } else {
            tracing::info!(%host, has_token = has, "the access token for this host was changed");
        }
    }

    /// Whether requests to the provider carry a token, and whether the service has refused it.
    pub fn token_state(&self, provider: Provider) -> TokenState {
        self.inner.creds.state(provider.host())
    }

    /// [`token_state`](Self::token_state) for an arbitrary host.
    pub fn host_token_state(&self, host: &str) -> TokenState {
        self.inner.creds.state(host)
    }

    /// Call `hook` with the host when a service refuses the token it was sent (once per token). It runs on a network thread: be
    /// quick.
    pub fn on_token_rejected(&self, hook: impl Fn(&str) + Send + Sync + 'static) {
        *self.inner.on_rejected.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(hook));
    }

    pub(crate) fn note_rejected(&self, host: &str) {
        if !self.inner.creds.reject(host) {
            return;
        }
        tracing::warn!(%host, "the service refused the access token; it is not sent any more and the request is repeated without it");
        let hook = self.inner.on_rejected.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(hook) = hook {
            hook(host);
        }
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
        let started = Instant::now();
        let result = self.block(self.fetch_async(request))?;
        log_fetch(request, &result, started);
        result
    }

    /// The saved copy of `request`, of any age, without touching the network. `None` when nothing was saved (or caching is off
    /// or the request has no cache rule). What an offline start has to show.
    pub fn cached(&self, request: &Request) -> Option<Fetched> {
        let url = self.parse(&request.url).ok()?;
        let cache = self.inner.cache.as_ref()?;
        request.cache?;
        let credential = self.inner.creds.tag(url.host_str().unwrap_or_default());
        let entry = cache.read(url.as_str(), &credential)?;
        let fresh = request.cache.is_some_and(|rule| entry.age_secs(unix_now()) < rule.ttl.as_secs());
        Some(from_entry(&entry, if fresh { Source::CacheFresh } else { Source::CacheStale }, None))
    }

    pub(crate) fn parse(&self, text: &str) -> Result<Url, NetError> {
        self.inner.cfg.address_policy.parse(text).map_err(NetError::Blocked)
    }

    /// The credential header for a request to `url`: only over https (or the test policy), only to the host that owns the token.
    pub(crate) fn auth_header(&self, url: &Url) -> Option<(HeaderName, HeaderValue)> {
        let secure = url.scheme() == "https" || self.inner.cfg.address_policy == crate::address::AddressPolicy::AnyHttpForTests;
        if !secure {
            return None;
        }
        self.inner.creds.header(url.host_str()?)
    }

    /// Wait for any other request for the same cached answer, so that two askers cost one question. `None` for a request that
    /// is not cached: there would be nothing for the second to read.
    async fn flight(&self, request: &Request, key: String) -> Option<Flight> {
        request.cache?;
        let lock = self.inner.flights.lock().unwrap_or_else(|e| e.into_inner()).entry(key.clone()).or_default().clone();
        let guard = lock.clone().lock_owned().await;
        Some(Flight { map: self.inner.flights.clone(), key, lock, _guard: guard })
    }

    async fn fetch_async(&self, request: &Request) -> Result<Fetched, NetError> {
        let url = self.parse(&request.url)?;
        let host = url.host_str().unwrap_or_default().to_string();
        let credential = self.inner.creds.tag(&host);
        let _flight = self.flight(request, format!("{credential}|{url}")).await;
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
                    if let Err(error) = cache.write(url.as_str(), &credential, &entry) {
                        tracing::warn!(url = %describe(&url), %error, "the answer could not be saved to the cache");
                    }
                }
                Ok(fetched)
            }
            Ok(Outcome::NotModified) => match saved {
                Some(mut entry) => {
                    entry.meta.fetched_at = unix_now();
                    if let Some(cache) = &self.inner.cache
                        && let Err(error) = cache.write(url.as_str(), &credential, &entry)
                    {
                        tracing::warn!(url = %describe(&url), %error, "the cache entry could not be refreshed");
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
                    tracing::warn!(
                        %host,
                        attempt = attempt + 1,
                        of = attempts,
                        wait_ms = wait.as_millis() as u64,
                        %error,
                        "request failed; trying again"
                    );
                    tokio::time::sleep(wait).await;
                }
                Err(Attempt::Retry { error, .. } | Attempt::Stop(error)) => return Err(error),
            }
        }
        Err(NetError::Other("no attempt was made".into()))
    }

    async fn once(&self, request: &Request, url: &Url, validators: Option<&Meta>) -> Result<Outcome, Attempt> {
        let host = url.host_str().unwrap_or_default().to_string();
        let mut builder = match &request.json {
            Some(body) => self.inner.api.post(url.clone()).header(CONTENT_TYPE, "application/json").body(body.clone()),
            None => self.inner.api.get(url.clone()),
        }
        .timeout(request.timeout);
        if let Some(accept) = &request.accept {
            builder = builder.header(ACCEPT, accept);
        }
        for (name, value) in &request.headers {
            builder = builder.header(name, value);
        }
        if let Some((name, value)) = self.auth_header(url) {
            builder = builder.header(name, value);
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
        if matches!(error, NetError::ProxyDenied { .. }) {
            // Once, not once per picture: the gate answers the next requests itself for POLICY_MEMORY.
            if self.inner.gate.blocked(host).is_none() {
                tracing::warn!(
                    %host,
                    minutes = POLICY_MEMORY.as_secs() / 60,
                    "the network (a proxy or firewall) refused this host; not asking again for a while"
                );
            }
            self.inner.gate.deny(host, error.clone(), POLICY_MEMORY);
        }
        if error.is_transient() { Attempt::Retry { error, wait: None } } else { Attempt::Stop(error) }
    }

    /// A non-success answer: a limit, a bot check, a server fault or a plain refusal.
    pub(crate) async fn rejected(&self, host: &str, response: Response) -> Attempt {
        let status = response.status().as_u16();
        // A token the service refuses is worse than none (GitHub answers 401 even for public data): stop sending it and ask again
        // as an anonymous client.
        if status == 401 && self.inner.creds.state(host) == TokenState::Active {
            self.note_rejected(host);
            return Attempt::Retry { error: NetError::Unauthorized { host: host.to_string() }, wait: Some(Duration::ZERO) };
        }
        let headers: HeaderMap = response.headers().clone();
        let peek = peek_body(response, ERROR_BODY_PEEK).await;
        let text = String::from_utf8_lossy(&peek).into_owned();
        if is_bot_challenge(status, &headers, &peek) {
            tracing::warn!(%host, status, "the server answered with a bot-check page instead of the data");
            return Attempt::Stop(NetError::BotChallenge { host: host.to_string(), status });
        }
        if let Some(wait) = limited_for(status, &headers, &text, std::time::SystemTime::now()) {
            let error = NetError::RateLimited { host: host.to_string(), retry_in: wait };
            tracing::warn!(
                %host,
                status,
                wait_s = wait.as_secs(),
                remaining = crate::limits::remaining(&headers),
                short_wait = wait <= SHORT_WAIT,
                "rate limit reached"
            );
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

/// One line per fetch: what, from where, how big, how long. A failure carries the hint the user would be shown.
fn log_fetch(request: &Request, result: &Result<Fetched, NetError>, started: Instant) {
    let ms = started.elapsed().as_millis() as u64;
    let url = Url::parse(&request.url).map_or_else(|_| "(invalid address)".to_string(), |u| describe(&u));
    match result {
        Ok(fetched) => {
            tracing::debug!(%url, status = fetched.status, bytes = fetched.body.len(), source = ?fetched.source, ms, "fetched");
            if let Some(error) = &fetched.stale_because {
                tracing::warn!(%url, %error, "the request failed, so the saved copy is shown");
            }
        }
        // A remembered refusal (rate limit, network policy) was logged when it was learned; repeating it per request is noise.
        Err(error @ (NetError::RateLimited { .. } | NetError::ProxyDenied { .. })) => tracing::debug!(%url, %error, "fetch refused"),
        Err(error) => tracing::warn!(%url, %error, hint = error.hint(), ms, "fetch failed"),
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
