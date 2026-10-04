//! A catalog list file: a named, versioned list of apps that one author publishes (`Nintendo.json`). Everything but
//! the apps is optional, and a list the format's older versions wrote (`standard`/`experimental`/`custom`, or a bare
//! array) still reads.
use std::cmp::Ordering;

use serde_json::Value;
use sha2::{Digest, Sha256};
use url::Url;

use crate::{
    entry::AppEntry,
    error::CatalogError,
    normalize,
    parse::{Mode, Skipped, parse_apps},
};

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CatalogList {
    /// `None` when the file gives none; the subscriber then keeps what it had or derives one from the address.
    pub name: Option<String>,
    /// `None` when absent, so a subscriber keeps its old text; `Some("")` clears it.
    pub description: Option<String>,
    /// The author's version, or, when the file has none, a hash of what the apps say (see [`content_hash`]).
    pub version: String,
    /// An absolute http(s) address, or none: anything else clears the icon rather than keeping a broken one.
    pub icon_url: Option<String>,
    pub featured_tags: Vec<String>,
    pub preferred_tag_filters: Vec<String>,
    pub hidden_tag_filters: Vec<String>,
    pub apps: Vec<AppEntry>,
    /// Entries that could not be read; the rest are still usable.
    pub skipped: Vec<Skipped>,
}

fn string_field(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key).and_then(Value::as_str).map(str::trim).map(str::to_string)
}

fn tag_list(object: &serde_json::Map<String, Value>, key: &str) -> Vec<String> {
    match object.get(key) {
        Some(Value::Array(items)) => normalize::tags(items.iter().filter_map(Value::as_str)),
        _ => Vec::new(),
    }
}

/// An icon address the launcher will accept: absolute, http or https. Returned in the parsed (canonical) spelling.
pub fn normalize_icon_url(text: Option<&str>) -> Option<String> {
    let url = Url::parse(text?.trim()).ok()?;
    matches!(url.scheme(), "http" | "https").then(|| url.to_string())
}

/// `.NET`'s ordinal, case-insensitive order: upper-case each character, compare UTF-16 code units. Rust's
/// `to_uppercase` can turn one character into several (`ß` to `SS`) where .NET keeps it, an edge that only changes
/// the order of the hash's input lines, never the data.
fn ordinal_ignore_case(a: &str, b: &str) -> Ordering {
    a.to_uppercase().encode_utf16().cmp(b.to_uppercase().encode_utf16())
}

/// A fingerprint of a list's content, for lists that publish no version: any change to what the apps say changes
/// it, and the order of the apps does not. Hex, upper case, 64 characters. `mods` are left out, as in the launcher
/// this format comes from, so a mod-source edit alone does not read as a catalog change.
pub fn content_hash(apps: &[AppEntry]) -> String {
    let mut lines: Vec<String> = apps
        .iter()
        .filter(|a| !a.identity_key().trim().is_empty())
        .map(|a| {
            [
                a.instance_key(),
                a.identity_key(),
                a.repository.trim().to_string(),
                a.name.clone(),
                a.project.clone().unwrap_or_default(),
                a.folder_name.clone(),
                a.install_path.clone().unwrap_or_default(),
                a.icon_url.clone().unwrap_or_default(),
                a.preferred_version.clone().unwrap_or_default(),
                a.skipped_update_version.clone().unwrap_or_default(),
                a.release_asset_filter.clone().unwrap_or_default(),
                a.tags.join(", "),
                a.files_to_add.join(", "),
            ]
            .join("|")
        })
        .collect();
    lines.sort_by(|a, b| ordinal_ignore_case(a, b));
    let digest = Sha256::digest(lines.join("\n").as_bytes());
    digest.iter().map(|b| format!("{b:02X}")).collect()
}

/// What a list is called when its file does not say: the file name without its extension, dashes as spaces
/// (`my-cool-list.json` is "my cool list"). Works on a web address or a path.
pub fn name_from_location(location: &str) -> String {
    let location = location.trim();
    let path = match Url::parse(location) {
        Ok(url) if matches!(url.scheme(), "http" | "https") => url.path().to_string(),
        _ => location.to_string(),
    };
    let file = path.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    stem.replace('-', " ").trim().to_string()
}

/// Read a list file's text.
pub fn parse_list(text: &str) -> Result<CatalogList, CatalogError> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let root: Value = serde_json::from_str(text).map_err(|e| CatalogError::Json(e.to_string()))?;
    let (apps, skipped) = parse_apps(&root, Mode::Lenient)?;
    let mut list = CatalogList { apps, skipped, ..Default::default() };
    if let Value::Object(object) = &root {
        list.name = string_field(object, "name").filter(|n| !n.is_empty());
        list.description = string_field(object, "description");
        list.icon_url = normalize_icon_url(object.get("iconUrl").and_then(Value::as_str));
        list.featured_tags = tag_list(object, "featuredTags");
        list.preferred_tag_filters = tag_list(object, "preferredTagFilters");
        list.hidden_tag_filters = tag_list(object, "hiddenTagFilters");
        // A number is not a version: the author must write it as text.
        list.version = string_field(object, "version").unwrap_or_default();
    }
    if list.version.is_empty() {
        list.version = content_hash(&list.apps);
    }
    Ok(list)
}

/// A version for a person to read: short ones as they are, long ones (hashes) as their first and last eight characters.
pub fn version_for_display(version: &str) -> String {
    let version = version.trim();
    if version.is_empty() {
        return "unknown".into();
    }
    let chars: Vec<char> = version.chars().collect();
    if chars.len() <= 16 {
        return version.to_string();
    }
    let (head, tail): (String, String) = (chars[..8].iter().collect(), chars[chars.len() - 8..].iter().collect());
    format!("{head}…{tail}")
}

#[cfg(test)]
mod tests;
