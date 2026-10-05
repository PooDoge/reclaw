//! Who the program is to the services that count requests per identity. A token raises GitHub's limit from 60 requests an hour to
//! 5,000, so it matters; it is also a secret, so this is the only place that holds one, hands it to a request, or decides it is
//! no longer good.
//!
//! The rules, each of which exists because the alternative went wrong somewhere:
//! * A token goes to its own service's API host and nowhere else, and never to where a redirect leads (a release asset is
//!   redirected to a signed storage URL, which rejects an extra `Authorization` header). It is sent as `Authorization: Bearer`
//!   for both services, because the HTTP client removes exactly that header when a redirect changes host; a custom header such
//!   as GitLab's `PRIVATE-TOKEN` (which Quiver uses) would follow the redirect.
//! * A token a service refuses (revoked, expired) is **not sent again**: GitHub answers `401 Bad credentials` even for public
//!   data, so a dead token is worse than none. The request is repeated without it, the token is kept so the person can see
//!   that it was refused, and a new one replaces it.
//! * Answers are cached per identity: what a token could see is never handed to a request without it.
use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
};

use reclaw_log::{Secret, forget_secret, register_secret};
use reqwest::header::{AUTHORIZATION, HeaderName, HeaderValue};
use sha2::{Digest, Sha256};

/// A service with its own accounts and limits.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Provider {
    GitHub,
    GitLab,
}

impl Provider {
    pub const ALL: [Provider; 2] = [Self::GitHub, Self::GitLab];

    /// The one host that receives this provider's token.
    pub fn host(self) -> &'static str {
        match self {
            Self::GitHub => "api.github.com",
            Self::GitLab => "gitlab.com",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
        }
    }

    pub fn from_host(host: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.host().eq_ignore_ascii_case(host))
    }

    /// The environment variables that supply a token, the program's own first.
    pub fn env_names(self) -> [&'static str; 2] {
        match self {
            Self::GitHub => ["RECLAW_GITHUB_TOKEN", "GITHUB_TOKEN"],
            Self::GitLab => ["RECLAW_GITLAB_TOKEN", "GITLAB_TOKEN"],
        }
    }

    /// The service's page for making a token, with the name filled in.
    pub fn token_page(self) -> &'static str {
        match self {
            Self::GitHub => "https://github.com/settings/tokens/new?description=Reclaw+launcher+rate+limit",
            Self::GitLab => "https://gitlab.com/-/user_settings/personal_access_tokens",
        }
    }

    /// An address that answers about the token (and, for GitHub, costs nothing against the limit).
    pub fn check_url(self) -> &'static str {
        match self {
            Self::GitHub => "https://api.github.com/rate_limit",
            Self::GitLab => "https://gitlab.com/api/v4/personal_access_tokens/self",
        }
    }
}

/// Whether requests to a host carry a token.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TokenState {
    /// There is none.
    None,
    /// There is one and it is sent.
    Active,
    /// There is one, the service refused it, and it is no longer sent.
    Rejected,
}

struct Entry {
    token: Secret,
    rejected: bool,
}

/// The tokens by host. Changed while the program runs (the person pastes one in Settings), read by every request.
pub(crate) struct Credentials {
    by_host: RwLock<HashMap<String, Entry>>,
}

impl Credentials {
    pub(crate) fn new(initial: &[(String, Secret)]) -> Self {
        let this = Self { by_host: RwLock::new(HashMap::new()) };
        for (host, token) in initial {
            this.set(host, Some(token.clone()));
        }
        this
    }

    /// Replace (or, with `None`, remove) the token for `host`. A new token starts out trusted.
    pub(crate) fn set(&self, host: &str, token: Option<Secret>) {
        let key = host.to_ascii_lowercase();
        let new_text = token.as_ref().map(|t| t.expose().to_string());
        let mut map = self.by_host.write().unwrap_or_else(PoisonError::into_inner);
        let old = match token {
            Some(token) => {
                register_secret(token.expose());
                map.insert(key, Entry { token, rejected: false })
            }
            None => map.remove(&key),
        };
        // Forget what was replaced, but not when it is the same text that was just registered: the same token set twice (the
        // environment's, applied again at start) must stay covered by the log's redaction.
        if let Some(old) = old
            && Some(old.token.expose()) != new_text.as_deref()
        {
            forget_secret(old.token.expose());
        }
    }

    pub(crate) fn state(&self, host: &str) -> TokenState {
        match self.by_host.read().unwrap_or_else(PoisonError::into_inner).get(&host.to_ascii_lowercase()) {
            None => TokenState::None,
            Some(entry) if entry.rejected => TokenState::Rejected,
            Some(_) => TokenState::Active,
        }
    }

    /// The header to send to `host`, if a trusted token is held for it. Marked sensitive so it never shows in debug output.
    pub(crate) fn header(&self, host: &str) -> Option<(HeaderName, HeaderValue)> {
        let map = self.by_host.read().unwrap_or_else(PoisonError::into_inner);
        let entry = map.get(&host.to_ascii_lowercase()).filter(|e| !e.rejected)?;
        let text = format!("Bearer {}", entry.token.expose());
        // A value with a character a header cannot hold is not a token anyone can use.
        let mut value = HeaderValue::from_str(&text).ok()?;
        value.set_sensitive(true);
        Some((AUTHORIZATION, value))
    }

    /// The service refused the token. `true` when this is news (it was trusted until now).
    pub(crate) fn reject(&self, host: &str) -> bool {
        let mut map = self.by_host.write().unwrap_or_else(PoisonError::into_inner);
        match map.get_mut(&host.to_ascii_lowercase()) {
            Some(entry) if !entry.rejected => {
                entry.rejected = true;
                true
            }
            _ => false,
        }
    }

    /// What separates one identity's cached answers from another's: `anon` when no token is sent, else a short fingerprint of the
    /// token (never the token).
    pub(crate) fn tag(&self, host: &str) -> String {
        let map = self.by_host.read().unwrap_or_else(PoisonError::into_inner);
        match map.get(&host.to_ascii_lowercase()).filter(|e| !e.rejected) {
            None => "anon".to_string(),
            Some(entry) => Sha256::digest(entry.token.expose().as_bytes()).iter().take(6).map(|b| format!("{b:02x}")).collect(),
        }
    }
}

#[cfg(test)]
mod tests;
