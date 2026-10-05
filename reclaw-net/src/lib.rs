//! The one HTTP client of the program, and what it needs to be a good citizen and a fast one: a user agent that says who
//! is asking, retries with a growing, jittered delay, conditional requests and an on-disk cache, per-host limits that
//! respect `Retry-After` and the services' rate-limit headers, resumable downloads that are hashed as they arrive, and
//! failures sorted into kinds a person can act on. No UI types, so everything is tested against a server on this machine.
//!
//! * `address`: which addresses may be fetched (https, standard port, nothing on the local network, also after redirects)
//! * `config`: [`NetConfig`], its defaults and the environment variables that adjust it
//! * `credentials`: [`Provider`] and the tokens, changeable while running, sent to their own host only, dropped once refused
//! * `check`: asking a service whether a token works and how many requests it allows
//! * `error`: [`NetError`], failures by kind (policy refusal, bot check, rate limit, TLS, DNS ...) with a hint for each
//! * `limits`, `challenge`, `retry`: pure rules about rate limits, bot-check pages and backoff
//! * `gate`: requests in flight per host, and the memory of "wait"
//! * `cache`: answers kept on disk, written whole
//! * `request`, `net`: [`Net`] and `fetch` for small answers
//! * `download`: `Net::download`, for files of any size: resumable, hashed, cancellable
//! * `diagnose`: what actually happens when each host the program needs is contacted from this machine
//! * `testing` (feature `testing`): a small HTTP server on this machine for tests
//!
//! A word on "CSP": a Content Security Policy is a response header that a *browser* enforces on a page. A program like this one
//! reads the header as text and nothing happens, so there is nothing to work around, and pretending to be a browser would not
//! change an answer. What does block a client is the network's egress policy, a bot-check page, a missing user agent or a spent
//! rate limit; those are what [`NetError`] tells apart.
pub mod address;
pub mod cache;
pub mod challenge;
pub mod check;
pub mod config;
pub mod credentials;
pub mod diagnose;
pub mod download;
pub mod error;
pub mod gate;
pub mod limits;
pub mod net;
pub mod request;
pub mod retry;
#[cfg(feature = "testing")]
pub mod testing;

pub use address::{AddressPolicy, UrlError};
pub use check::{Quota, TokenInfo, Verdict};
pub use config::{NetConfig, PROJECT_URL, ProxyMode};
pub use credentials::{Provider, TokenState};
pub use download::{Cancel, DownloadRequest, Downloaded, Progress};
pub use error::NetError;
pub use net::Net;
pub use request::{CacheRule, Fetched, Request, Source};
