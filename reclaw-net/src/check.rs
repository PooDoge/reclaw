//! Asking a service whether a token works, and what it allows. For GitHub this is `GET /rate_limit`, which costs nothing against
//! the limit and answers the same question anonymously, so the Settings page can show "60 an hour, 58 left" before any token exists.
//! The result is also how a bad token is found out at the moment it is pasted, not when a download fails an hour later.
use std::time::Duration;

use reqwest::header::{ACCEPT, HeaderMap};
use serde::Deserialize;

use crate::{
    credentials::Provider,
    error::NetError,
    net::{Attempt, Net},
};

/// How many requests a service allows in its window, how many are left, and when the window resets (unix seconds).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quota {
    pub limit: u64,
    pub remaining: u64,
    pub reset_at: u64,
}

/// What a service said about the token it was sent (or about the anonymous client, when none was).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TokenInfo {
    /// Whether a token was actually sent.
    pub had_token: bool,
    pub quota: Option<Quota>,
    /// When the token stops working, as the service writes it, if it says.
    pub expires: Option<String>,
    /// The permissions of a GitHub classic token (`X-OAuth-Scopes`). A token for this program needs none, so a non-empty list is
    /// worth a warning. `None` for a token that has no such header (a fine-grained one) or no token.
    pub scopes: Option<Vec<String>>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Verdict {
    Accepted(TokenInfo),
    /// The service refused the token. It is no longer sent.
    Rejected,
}

const CHECK_TIMEOUT: Duration = Duration::from_secs(15);
const CHECK_LIMIT: usize = 64 * 1024;

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

fn number(headers: &HeaderMap, names: &[&str]) -> Option<u64> {
    names.iter().find_map(|n| header(headers, n)?.trim().parse().ok())
}

/// The quota a response states in its headers (`x-ratelimit-*` on GitHub, `ratelimit-*` on GitLab).
fn quota_from_headers(headers: &HeaderMap) -> Option<Quota> {
    Some(Quota {
        limit: number(headers, &["x-ratelimit-limit", "ratelimit-limit"])?,
        remaining: number(headers, &["x-ratelimit-remaining", "ratelimit-remaining"])?,
        reset_at: number(headers, &["x-ratelimit-reset", "ratelimit-reset"]).unwrap_or(0),
    })
}

#[derive(Deserialize)]
struct GithubLimits {
    resources: Option<GithubResources>,
}

#[derive(Deserialize)]
struct GithubResources {
    core: Option<GithubWindow>,
}

#[derive(Deserialize)]
struct GithubWindow {
    limit: u64,
    remaining: u64,
    reset: u64,
}

#[derive(Deserialize)]
struct GitlabToken {
    active: Option<bool>,
    revoked: Option<bool>,
    expires_at: Option<String>,
}

fn github_info(headers: &HeaderMap, body: &[u8], had_token: bool) -> TokenInfo {
    // The body is the authority; the headers say the same about the request just made and are the fallback.
    let from_body = serde_json::from_slice::<GithubLimits>(body).ok().and_then(|l| l.resources?.core).map(|w| Quota {
        limit: w.limit,
        remaining: w.remaining,
        reset_at: w.reset,
    });
    let scopes =
        header(headers, "x-oauth-scopes").map(|s| s.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect());
    TokenInfo {
        had_token,
        quota: from_body.or_else(|| quota_from_headers(headers)),
        expires: header(headers, "github-authentication-token-expiration").map(|s| s.trim().to_string()).filter(|s| !s.is_empty()),
        scopes: if had_token { scopes } else { None },
    }
}

fn gitlab_verdict(headers: &HeaderMap, body: &[u8]) -> Verdict {
    match serde_json::from_slice::<GitlabToken>(body) {
        Ok(token) if token.active == Some(false) || token.revoked == Some(true) => Verdict::Rejected,
        Ok(token) => {
            Verdict::Accepted(TokenInfo { had_token: true, quota: quota_from_headers(headers), expires: token.expires_at, scopes: None })
        }
        // A 200 that is not the shape we know still means the service accepted the token.
        Err(_) => Verdict::Accepted(TokenInfo { had_token: true, quota: quota_from_headers(headers), ..TokenInfo::default() }),
    }
}

impl Net {
    /// Ask `provider` about the token that is being sent to it. Call from a worker thread. A refused token is reported as
    /// [`Verdict::Rejected`] and from then on not sent. With no token, GitHub still answers with the anonymous allowance;
    /// GitLab has nothing to say, and that is an `Unauthorized` error.
    pub fn check_token(&self, provider: Provider) -> Result<Verdict, NetError> {
        self.check_at(provider, provider.check_url())
    }

    /// [`check_token`](Self::check_token) against another address: for tests against a server on this machine.
    pub fn check_at(&self, provider: Provider, url: &str) -> Result<Verdict, NetError> {
        self.block(self.check_async(provider, url))?
    }

    async fn check_async(&self, provider: Provider, url: &str) -> Result<Verdict, NetError> {
        let url = self.parse(url)?;
        let host = url.host_str().unwrap_or_default().to_string();
        let auth = self.auth_header(&url);
        let had_token = auth.is_some();
        let permit = self.inner.gate.enter(&host).await?;
        let mut builder =
            self.inner.api.get(url.clone()).timeout(CHECK_TIMEOUT).header(ACCEPT, "application/vnd.github+json, application/json");
        if provider == Provider::GitHub {
            builder = builder.header("X-GitHub-Api-Version", "2022-11-28");
        }
        if let Some((name, value)) = auth {
            builder = builder.header(name, value);
        }
        let response = builder.send().await.map_err(|e| match self.transport(&host, &e) {
            Attempt::Retry { error, .. } | Attempt::Stop(error) => error,
        })?;
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        if status == 401 {
            drop(permit);
            return if had_token {
                self.note_rejected(&host);
                Ok(Verdict::Rejected)
            } else {
                Err(NetError::Unauthorized { host })
            };
        }
        if !response.status().is_success() {
            let attempt = self.rejected(&host, response).await;
            drop(permit);
            return Err(match attempt {
                Attempt::Retry { error, .. } | Attempt::Stop(error) => error,
            });
        }
        let mut response = response;
        let mut body = Vec::new();
        while body.len() < CHECK_LIMIT {
            match response.chunk().await {
                Ok(Some(chunk)) => body.extend_from_slice(&chunk),
                Ok(None) => break,
                Err(e) => {
                    return Err(match self.transport(&host, &e) {
                        Attempt::Retry { error, .. } | Attempt::Stop(error) => error,
                    });
                }
            }
        }
        drop(permit);
        let verdict = match provider {
            Provider::GitHub => Verdict::Accepted(github_info(&headers, &body, had_token)),
            Provider::GitLab => gitlab_verdict(&headers, &body),
        };
        if let Verdict::Rejected = verdict {
            self.note_rejected(&host);
        }
        tracing::debug!(%host, ?verdict, "token checked");
        Ok(verdict)
    }
}

#[cfg(test)]
mod tests;
