//! Why a request failed, in the words a person can act on. The kinds matter more than the text: a refusal by the
//! network's policy, a site asking for a browser and a rate limit all look like "403" to a client that only reads status
//! codes, and each needs a different answer.
use std::time::Duration;

use crate::address::UrlError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NetError {
    /// The address policy refused it (see `address`).
    #[error("{0}")]
    Blocked(UrlError),
    #[error("could not find {host} (DNS lookup failed)")]
    Dns { host: String },
    #[error("could not connect to {host}: {detail}")]
    Connect { host: String, detail: String },
    /// A proxy or the network's policy refused to open a tunnel to the host. Not something a client can fix.
    #[error("the network refused to connect to {host}{}", status.map(|s| format!(" ({s})")).unwrap_or_default())]
    ProxyDenied { host: String, status: Option<u16> },
    #[error("could not make a secure connection to {host}: {detail}")]
    Tls { host: String, detail: String },
    #[error("timed out")]
    Timeout,
    #[error("no data arrived for {} seconds", .0.as_secs())]
    Stalled(Duration),
    #[error("{host} answered {status}")]
    Status { host: String, status: u16 },
    #[error("{host} is limiting requests; try again in {} seconds", retry_in.as_secs())]
    RateLimited { host: String, retry_in: Duration },
    /// The site answered with a page meant for a browser to solve. Reclaw says who it is and does not pretend to be one.
    #[error("{host} is asking for a web browser (a bot check, status {status})")]
    BotChallenge { host: String, status: u16 },
    #[error("larger than the {limit} byte limit")]
    TooLarge { limit: u64 },
    #[error("cancelled")]
    Cancelled,
    #[error("the disk is full")]
    DiskFull,
    #[error("{0}")]
    Io(String),
    #[error("the download is not what was promised: {0}")]
    Integrity(String),
    #[error("{0}")]
    Other(String),
    /// A blocking call was made from inside an async runtime, where it would deadlock.
    #[error("a blocking network call was made from inside an async runtime")]
    WrongContext,
}

impl NetError {
    /// Worth trying again: the failure says nothing about the request itself.
    pub fn is_transient(&self) -> bool {
        match self {
            Self::Connect { .. } | Self::Timeout | Self::Stalled(_) | Self::Dns { .. } => true,
            Self::Status { status, .. } => matches!(status, 408 | 425 | 500 | 502 | 503 | 504),
            _ => false,
        }
    }

    /// The network is unreachable or unwell, as opposed to the server rejecting this request. Reasonable grounds to
    /// show a saved copy instead.
    pub fn is_connectivity(&self) -> bool {
        matches!(
            self,
            Self::Dns { .. }
                | Self::Connect { .. }
                | Self::ProxyDenied { .. }
                | Self::Tls { .. }
                | Self::Timeout
                | Self::Stalled(_)
                | Self::RateLimited { .. }
                | Self::BotChallenge { .. }
        ) || matches!(self, Self::Status { status, .. } if *status >= 500)
    }

    /// One sentence on what to do about it, or nothing when there is nothing to do.
    pub fn hint(&self) -> Option<&'static str> {
        Some(match self {
            Self::ProxyDenied { .. } => {
                "A proxy or firewall refused the connection. Allow this host there (Reclaw does not route around it)."
            }
            Self::BotChallenge { .. } => {
                "The site wants a real browser. Reclaw identifies itself honestly and does not imitate one; use the site's API with a token, or a mirror."
            }
            Self::RateLimited { .. } => "Wait, or add an access token (for GitHub, set GITHUB_TOKEN) to raise the limit.",
            Self::Tls { .. } => "The certificate was not trusted. On a network that re-signs traffic, set SSL_CERT_FILE to its CA bundle.",
            Self::Dns { .. } => "Check the network connection and DNS settings.",
            Self::DiskFull => "Free some disk space and try again; the partial download is kept.",
            Self::WrongContext => "This is a bug in the caller: use the async method.",
            _ => return None,
        })
    }
}

/// Every message in an error's chain of causes, outermost first, joined. `reqwest` wraps the cause that matters
/// (a refused tunnel, a certificate problem) several layers down and offers no accessor for it.
pub(crate) fn chain_text(error: &(dyn std::error::Error + 'static)) -> String {
    let mut parts = vec![error.to_string()];
    let mut source = error.source();
    while let Some(cause) = source {
        parts.push(cause.to_string());
        source = cause.source();
    }
    parts.join(": ")
}

/// Turn the text of a failure into a kind. Pure, so the patterns are tested without a network.
pub(crate) fn classify_text(host: &str, text: &str, is_timeout: bool, is_connect: bool) -> NetError {
    let lower = text.to_ascii_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| lower.contains(n));
    if has(&["unsuccessful tunnel", "proxy authentication required", "tunnel error", "proxy connect", "connect response"]) {
        let status = ["403", "407", "502", "503"].iter().find(|s| lower.contains(*s)).and_then(|s| s.parse().ok());
        return NetError::ProxyDenied { host: host.to_string(), status };
    }
    if has(&["certificate", "invalid peer", "unknown issuer", "tls handshake", "handshake failure", "alert", "rustls", "ssl"]) {
        return NetError::Tls { host: host.to_string(), detail: first_cause(text) };
    }
    if has(&[
        "dns error",
        "failed to lookup",
        "no such host",
        "name or service not known",
        "temporary failure in name resolution",
        "nodename nor servname",
    ]) {
        return NetError::Dns { host: host.to_string() };
    }
    if is_timeout || has(&["timed out", "deadline has elapsed"]) {
        return NetError::Timeout;
    }
    if is_connect
        || has(&[
            "connection refused",
            "connection reset",
            "network is unreachable",
            "no route to host",
            "broken pipe",
            "connection closed",
            "error trying to connect",
        ])
    {
        return NetError::Connect { host: host.to_string(), detail: first_cause(text) };
    }
    NetError::Other(text.to_string())
}

/// The innermost message of a joined chain: usually the one that says what happened.
fn first_cause(text: &str) -> String {
    text.rsplit(": ").next().unwrap_or(text).to_string()
}

pub(crate) fn from_reqwest(host: &str, error: &reqwest::Error) -> NetError {
    let text = chain_text(error);
    if error.is_redirect() {
        return NetError::Other(text);
    }
    classify_text(host, &text, error.is_timeout(), error.is_connect())
}

pub(crate) fn from_io(error: &std::io::Error) -> NetError {
    match error.kind() {
        std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded => NetError::DiskFull,
        _ => NetError::Io(error.to_string()),
    }
}

#[cfg(test)]
mod tests;
