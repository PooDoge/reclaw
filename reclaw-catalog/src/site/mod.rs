//! The quiverlauncher.com catalog, which replaced the community lists as Quiver's catalog on 2026-10-09 (the lists stay in place,
//! frozen, "so existing installs keep working"). Plain data: where each answer is asked for, what it looks like, and which entry a
//! catalog or library app is. Asking is `reclaw-sync`'s job.
//!
//! * `app.rs`: an app as the catalog lists it (what it is, how it installs, its art, its ratings and releases)
//! * `entry.rs`: a listed app as a catalog entry in Quiver's list format (`AppEntry`), so the rest of Reclaw installs it as any other
//! * `follow.rs`: keeping a library app's name, project, icon and tags in step with its entry, as Quiver 3.5 does
//! * `types.rs`: the other answers (its page, a review, a release with its state and VirusTotal verdict, the release status feed)
//! * `link.rs`: which site entry an app is (its entry id, its `catalogId`, its repository, then its download filter or folder)
//! * this file: the addresses, and reading a page of a list leniently
mod app;
mod entry;
mod follow;
mod link;
mod types;

use serde::de::DeserializeOwned;
use serde_json::Value;

pub use app::*;
pub use entry::{folder_for, to_entry};
pub use follow::follow;
pub use link::{Links, link};
pub use types::*;

/// The public API. Every call is a plain GET; nothing needs a token.
pub const API: &str = "https://api.quiverlauncher.com/api/v1";
/// The same API on the hosting deployment's own address. `api.quiverlauncher.com` is a custom domain in front of it, and when that
/// domain's TLS handshake fails for a player this one still answers (Quiver 3.5 falls back to it the same way).
pub const FALLBACK_API: &str = "https://famous-wildebeest-660.convex.site/api/v1";
pub const WEBSITE: &str = "https://quiverlauncher.com";
/// The most items the API returns in one page.
pub const MAX_PAGE: u32 = 100;
/// How many reviews an app's page shows (Quiver shows ten).
pub const REVIEWS_SHOWN: u32 = 10;

/// Percent-encodes everything but the unreserved characters, for a path segment or a query value.
fn escape(text: &str) -> String {
    // `.` and `..` would be read as path segments (even percent-encoded) and send the request to another endpoint.
    if text.bytes().all(|b| b == b'.') {
        return "_".to_string();
    }
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn cursor_part(cursor: Option<&str>) -> String {
    cursor.map(|c| format!("&cursor={}", escape(c))).unwrap_or_default()
}

/// Paths under [`API`].
pub mod path {
    use super::{MAX_PAGE, cursor_part, escape};

    /// Every app, a page at a time, in the site's default order.
    pub fn apps(cursor: Option<&str>) -> String {
        format!("/apps?limit={MAX_PAGE}{}", cursor_part(cursor))
    }

    /// Every app's id, slug and repository: what links an app to its entry.
    pub fn release_status(cursor: Option<&str>) -> String {
        format!("/release-status?limit={MAX_PAGE}{}", cursor_part(cursor))
    }

    pub fn detail(slug: &str) -> String {
        format!("/apps/{}", escape(slug))
    }

    /// What players said, newest first.
    pub fn reviews(slug: &str, limit: u32) -> String {
        format!("/apps/{}/reviews?limit={limit}", escape(slug))
    }

    /// Where a launcher reports a download of one of the app's files that failed (POST).
    pub fn download_problem(slug: &str) -> String {
        format!("/apps/{}/download-problem", escape(slug))
    }

    /// Every release of the app's repository, newest first, each with its state.
    pub fn release_history(slug: &str) -> String {
        format!("/apps/{}/release-history?limit={MAX_PAGE}", escape(slug))
    }
}

/// The app's page on the website.
pub fn app_page_url(slug: &str) -> String {
    format!("{WEBSITE}/apps/{}", escape(slug))
}

/// Where a player says how the app ran for them (reviews are written on the website, not in a launcher).
pub fn review_page_url(slug: &str) -> String {
    format!("{}?tab=how-it-runs", app_page_url(slug))
}

/// One page of a list: its items, and where the next page starts (`None` on the last page).
#[derive(Clone, PartialEq, Debug)]
pub struct Page<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    /// Items that could not be read and were left out.
    pub skipped: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("the catalog's answer could not be read: {0}")]
pub struct ReadError(pub String);

/// A page of `{items, nextCursor, isDone}`. Lenient like a catalog list: an item that cannot be read is left out and counted, so one
/// odd entry does not hide the rest. A page that is not that shape at all is an error.
pub fn parse_page<T: DeserializeOwned>(text: &str) -> Result<Page<T>, ReadError> {
    let root: Value = serde_json::from_str(text).map_err(|e| ReadError(e.to_string()))?;
    let Some(items) = root.get("items").and_then(Value::as_array) else {
        return Err(ReadError("there is no list of items".into()));
    };
    let total = items.len();
    let items: Vec<T> = items.iter().filter_map(|item| serde_json::from_value(item.clone()).ok()).collect();
    let done = root.get("isDone").and_then(Value::as_bool).unwrap_or(true);
    let next = root.get("nextCursor").and_then(Value::as_str).filter(|c| !c.is_empty()).map(str::to_string);
    Ok(Page { skipped: total - items.len(), next_cursor: if done { None } else { next }, items })
}

/// One document (an app's page). `null` (what the API answers for something it does not have) is `None`.
pub fn parse_one<T: DeserializeOwned>(text: &str) -> Result<Option<T>, ReadError> {
    let root: Value = serde_json::from_str(text).map_err(|e| ReadError(e.to_string()))?;
    if root.is_null() {
        return Ok(None);
    }
    serde_json::from_value(root).map(Some).map_err(|e| ReadError(e.to_string()))
}

#[cfg(test)]
mod tests;
