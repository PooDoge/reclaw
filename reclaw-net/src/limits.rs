//! What a response says about being asked to slow down. GitHub and GitLab report a limit in headers on every
//! response (`x-ratelimit-*`, `ratelimit-*`) and answer 403 or 429 when it is spent; a busy server says `Retry-After`.
//! Pure functions over headers, so every rule has a test.
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::HeaderMap;

/// How long to wait when a server says it is limiting but not for how long.
pub const DEFAULT_WAIT: Duration = Duration::from_secs(60);
/// The longest wait honoured. A server asking for more gets told the request failed; sleeping for an hour in a launcher is not help.
pub const MAX_WAIT: Duration = Duration::from_secs(60 * 60);

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok()).map(str::trim)
}

fn number(headers: &HeaderMap, name: &str) -> Option<u64> {
    header(headers, name)?.parse().ok()
}

fn unix(now: SystemTime) -> u64 {
    now.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// `Retry-After`: a number of seconds, or an HTTP date.
pub fn retry_after(headers: &HeaderMap, now: SystemTime) -> Option<Duration> {
    let value = header(headers, "retry-after")?;
    if let Ok(secs) = value.parse::<u64>() {
        return Some(Duration::from_secs(secs));
    }
    let at = httpdate::parse_http_date(value).ok()?;
    Some(at.duration_since(now).unwrap_or(Duration::ZERO))
}

/// Requests left in the current window, if the server says (`x-ratelimit-remaining`, or GitLab's `ratelimit-remaining`).
pub fn remaining(headers: &HeaderMap) -> Option<u64> {
    number(headers, "x-ratelimit-remaining").or_else(|| number(headers, "ratelimit-remaining"))
}

/// Seconds until the window resets, from the unix time both services send.
pub fn reset_in(headers: &HeaderMap, now: SystemTime) -> Option<Duration> {
    let at = number(headers, "x-ratelimit-reset").or_else(|| number(headers, "ratelimit-reset"))?;
    Some(Duration::from_secs(at.saturating_sub(unix(now))))
}

/// Whether this answer means "slow down", and for how long. `body` is the start of the response body, for services that
/// say it only there.
///
/// * 429 always does.
/// * 403 does when the window is spent (`remaining` 0), when the server sent `Retry-After`, or when the body says so:
///   GitHub's secondary limits arrive as a plain 403.
/// * 503 does only with a `Retry-After`: otherwise it is an outage, which is retried differently.
pub fn limited_for(status: u16, headers: &HeaderMap, body: &str, now: SystemTime) -> Option<Duration> {
    let wait = retry_after(headers, now).or_else(|| reset_in(headers, now));
    let says_so = body.to_ascii_lowercase().contains("rate limit");
    let limited = match status {
        429 => true,
        403 => remaining(headers) == Some(0) || retry_after(headers, now).is_some() || says_so,
        503 => retry_after(headers, now).is_some(),
        _ => false,
    };
    limited.then(|| wait.unwrap_or(DEFAULT_WAIT).min(MAX_WAIT))
}

#[cfg(test)]
mod tests;
