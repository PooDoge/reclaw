//! A release and its files, as GitHub and GitLab describe them, and which release to install. One shape for both hosts, so
//! nothing after parsing knows which one it came from.
use serde_json::Value;

use crate::version;

/// One downloadable file of a release.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
    /// Lower-case hex SHA-256, when the host states it (GitHub does for newer uploads).
    pub sha256: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Release {
    pub tag: String,
    pub name: String,
    pub prerelease: bool,
    /// The host's own text for the release (markdown); shown as "what changed".
    pub notes: String,
    pub published: String,
    pub page: String,
    pub assets: Vec<Asset>,
}

fn text(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().trim().to_string()
}

fn github_asset(value: &Value) -> Option<Asset> {
    let name = text(value, "name");
    let url = text(value, "browser_download_url");
    if name.is_empty() || url.is_empty() {
        return None;
    }
    let sha256 = value.get("digest").and_then(Value::as_str).and_then(|d| d.strip_prefix("sha256:")).map(str::to_ascii_lowercase);
    Some(Asset { name, url, size: value.get("size").and_then(Value::as_u64), sha256 })
}

/// A release in GitHub's shape. A draft is not a release anyone can download; a tagless entry is not usable.
pub fn parse_github_release(value: &Value) -> Option<Release> {
    let tag = text(value, "tag_name");
    if tag.is_empty() || value.get("draft").and_then(Value::as_bool).unwrap_or(false) {
        return None;
    }
    let assets = value.get("assets").and_then(Value::as_array).map(|a| a.iter().filter_map(github_asset).collect()).unwrap_or_default();
    Some(Release {
        name: text(value, "name"),
        prerelease: value.get("prerelease").and_then(Value::as_bool).unwrap_or(false),
        notes: text(value, "body"),
        published: text(value, "published_at"),
        page: text(value, "html_url"),
        assets,
        tag,
    })
}

/// A GitLab link is only a download when it points at GitLab itself: a release can link anywhere, and following a link to
/// another host from here would send our token's neighbours a request they never asked for.
fn is_gitlab_hosted(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or_default().split('@').next_back().unwrap_or_default();
    let host = host.split(':').next().unwrap_or_default().to_ascii_lowercase();
    host == "gitlab.com" || host.ends_with(".gitlab.com")
}

fn gitlab_asset(link: &Value) -> Option<Asset> {
    let name = text(link, "name");
    let direct = text(link, "direct_asset_url");
    let url = if direct.is_empty() { text(link, "url") } else { direct };
    (!name.is_empty() && is_gitlab_hosted(&url)).then_some(Asset { name, url, size: None, sha256: None })
}

/// A release in GitLab's shape: its files are the release's links (the generated source archives are not offered).
pub fn parse_gitlab_release(value: &Value) -> Option<Release> {
    let tag = text(value, "tag_name");
    if tag.is_empty() {
        return None;
    }
    let assets = value
        .pointer("/assets/links")
        .and_then(Value::as_array)
        .map(|links| links.iter().filter_map(gitlab_asset).collect())
        .unwrap_or_default();
    Some(Release {
        name: text(value, "name"),
        prerelease: value.get("upcoming_release").and_then(Value::as_bool).unwrap_or(false),
        notes: text(value, "description"),
        published: text(value, "released_at"),
        page: value.pointer("/_links/self").and_then(Value::as_str).unwrap_or_default().to_string(),
        assets,
        tag,
    })
}

/// Releases from a list answer; entries that are not releases are skipped.
pub fn parse_list(body: &[u8], parse: fn(&Value) -> Option<Release>) -> Result<Vec<Release>, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("the release list is not JSON: {e}"))?;
    match value {
        Value::Array(items) => Ok(items.iter().filter_map(parse).collect()),
        // `/releases/latest` answers with one release, not a list.
        Value::Object(_) => Ok(parse(&value).into_iter().collect()),
        _ => Err("the release list is neither a list nor a release".to_string()),
    }
}

impl Release {
    /// Files worth offering: not metadata, checksums or source archives.
    pub fn downloadable(&self, filter: Option<&str>) -> Vec<&Asset> {
        let filter = filter.map(str::trim).filter(|f| !f.is_empty()).map(str::to_ascii_lowercase);
        self.assets
            .iter()
            .filter(|a| !crate::names::is_auxiliary(&a.name.to_ascii_lowercase()))
            .filter(|a| filter.as_ref().is_none_or(|f| a.name.to_ascii_lowercase().contains(f)))
            .collect()
    }

    pub fn has_assets(&self) -> bool {
        !self.downloadable(None).is_empty()
    }
}

fn find_by_tag<'a>(releases: &'a [Release], tag: &str) -> Option<&'a Release> {
    releases.iter().find(|r| version::equivalent(&r.tag, tag))
}

/// Which release to install or compare against.
///
/// A pinned version wins. With `allow_prerelease`, the newest release with files, stable or not (the lists are newest first).
/// Otherwise the tag the host calls "latest", if it has files; then the newest stable release with files; and only when there
/// is none the newest pre-release.
pub fn select_release<'a>(
    releases: &'a [Release],
    preferred: Option<&str>,
    host_latest_tag: Option<&str>,
    allow_prerelease: bool,
) -> Option<&'a Release> {
    if let Some(tag) = preferred.filter(|t| !t.trim().is_empty())
        && let Some(pinned) = find_by_tag(releases, tag)
    {
        return Some(pinned);
    }
    if allow_prerelease {
        return releases.iter().find(|r| r.has_assets());
    }
    if let Some(tag) = host_latest_tag.filter(|t| !t.trim().is_empty())
        && let Some(flagged) = find_by_tag(releases, tag).filter(|r| r.has_assets() && !r.prerelease)
    {
        return Some(flagged);
    }
    releases.iter().find(|r| !r.prerelease && r.has_assets()).or_else(|| releases.iter().find(|r| r.prerelease && r.has_assets()))
}
