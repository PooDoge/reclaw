//! Noticing that a site answered with a page meant for a browser to solve (a "checking your browser" interstitial or a
//! CAPTCHA) and not with the data asked for. A plain 403 would hide the difference. Reclaw does not try to pass these:
//! it identifies itself honestly, reports the situation, and leaves the choice (an API token, a mirror, another source)
//! to the person.
use reqwest::header::HeaderMap;

/// Markers that appear in challenge pages. Lower case; the body is lowered before comparing.
const BODY_MARKERS: &[&str] = &[
    "just a moment...",
    "cf-chl",
    "challenge-platform",
    "attention required! | cloudflare",
    "checking your browser",
    "captcha",
    "ddos-guard",
    "_incapsula_",
];

/// Whether an answer is a challenge page. `body` is the start of the body (a few hundred bytes are enough).
///
/// Cloudflare marks them with `cf-mitigated: challenge`; other services are recognised by their page text, and only on the
/// statuses such pages come with, so a normal page that mentions a CAPTCHA does not count.
pub fn is_bot_challenge(status: u16, headers: &HeaderMap, body: &[u8]) -> bool {
    if !matches!(status, 401 | 403 | 429 | 503) {
        return false;
    }
    let value = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_ascii_lowercase();
    if value("cf-mitigated").contains("challenge") {
        return true;
    }
    let head = String::from_utf8_lossy(&body[..body.len().min(4096)]).to_ascii_lowercase();
    let looks_like_html = head.contains("<html") || head.contains("<!doctype") || head.contains("<title");
    looks_like_html && BODY_MARKERS.iter().any(|m| head.contains(m))
}

#[cfg(test)]
mod tests;
