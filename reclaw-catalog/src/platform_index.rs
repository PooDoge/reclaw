//! The platform index (`platform-index.json`): a generated file saying which release of each catalog repository was
//! looked at and what files it held, so a launcher can tell which apps run on this machine without asking the hosting
//! service about every one. It is evidence about downloads, never an instruction to download.
//!
//! A document is valid whole or not at all: one bad entry rejects the file and the previous good copy stays in use.
use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::{
    error::CatalogError,
    json::{field, text, whole_number},
    source::RepoSource,
    timestamp::Timestamp,
};

/// The largest body a launcher should read for this file.
pub const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_ENTRIES: usize = 10_000;
const MAX_ASSETS: usize = 10_000;
const MAX_ASSET_NAME_UTF16: usize = 2048;
/// How far ahead of the document's own clock an entry's check, or the document itself, may claim to be.
pub const CLOCK_SKEW_SECS: i64 = 5 * 60;
/// How long a result checked at revision 2 or later counts as current.
pub const FRESH_FOR_SECS: i64 = 24 * 60 * 60;

/// What identifies one result: the service, the repository (case-insensitive on GitHub, exact on GitLab) and the
/// release the app is pinned to. An unpinned app asks about the release the launcher would pick, which is `None`.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct PlatformKey {
    source: RepoSource,
    repository: String,
    preferred: String,
}

impl PlatformKey {
    pub fn new(source: RepoSource, repository: &str, preferred_release: Option<&str>) -> Self {
        let repository = repository.trim();
        let repository = if source == RepoSource::Github { repository.to_lowercase() } else { repository.to_string() };
        Self { source, repository, preferred: preferred_release.map(str::trim).unwrap_or_default().to_string() }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlatformEntry {
    pub source: RepoSource,
    pub repository: String,
    pub preferred_release: Option<String>,
    /// The release looked at. Empty with no assets means "checked, nothing usable".
    pub release_tag: String,
    pub asset_names: Vec<String>,
    pub validated_at: Timestamp,
    /// Which version of the selection rules produced this. 1 or 2.
    pub selection_revision: u8,
}

impl PlatformEntry {
    pub fn key(&self) -> PlatformKey {
        PlatformKey::new(self.source, &self.repository, self.preferred_release.as_deref())
    }

    /// Current enough to skip asking again: made by revision 2 of the rules or later, and under a day old. Revision 1
    /// results stay usable but are never fresh, so a launcher confirms them once when it can.
    pub fn is_fresh(&self, now: Timestamp) -> bool {
        self.selection_revision >= 2 && now.secs.saturating_sub(self.validated_at.secs) < FRESH_FOR_SECS
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlatformDocument {
    pub format_revision: u8,
    pub generated_at: Timestamp,
    pub entries: Vec<PlatformEntry>,
}

fn invalid(entry: usize, message: impl std::fmt::Display) -> CatalogError {
    CatalogError::Shape(format!("platform entry {}: {message}", entry + 1))
}

fn timestamp(object: &Map<String, Value>, name: &str, owner: &str) -> Result<Timestamp, CatalogError> {
    let raw = text(object, name)?.ok_or_else(|| CatalogError::shape(format!("{owner}: {name} is missing")))?;
    let at = Timestamp::parse(&raw)
        .ok_or_else(|| CatalogError::shape(format!("{owner}: {name} \"{raw}\" is not a date and time with an offset")))?;
    if at.is_default() {
        return Err(CatalogError::shape(format!("{owner}: {name} is missing")));
    }
    Ok(at)
}

impl PlatformEntry {
    fn from_value(value: &Value, index: usize, generated_at: Timestamp) -> Result<Self, CatalogError> {
        let Value::Object(object) = value else {
            return Err(invalid(index, "is not an object"));
        };
        let owner = format!("platform entry {}", index + 1);
        let source = match text(object, "provider")?.as_deref() {
            Some("github") => RepoSource::Github,
            Some("gitlab") => RepoSource::Gitlab,
            other => return Err(invalid(index, format!("provider must be exactly \"github\" or \"gitlab\", not {other:?}"))),
        };
        let repository =
            text(object, "repository")?.filter(|r| !r.trim().is_empty()).ok_or_else(|| invalid(index, "repository is missing"))?;
        let release_tag = text(object, "releaseTag")?.ok_or_else(|| invalid(index, "releaseTag is missing (an empty one is allowed)"))?;
        let asset_names = match field(object, "assetNames") {
            Some(Value::Array(items)) => {
                if items.len() > MAX_ASSETS {
                    return Err(invalid(index, format!("more than {MAX_ASSETS} assets")));
                }
                let mut names = Vec::with_capacity(items.len());
                for item in items {
                    let name = item.as_str().ok_or_else(|| invalid(index, "an asset name is not a string"))?;
                    if name.encode_utf16().count() > MAX_ASSET_NAME_UTF16 {
                        return Err(invalid(index, format!("an asset name is longer than {MAX_ASSET_NAME_UTF16} characters")));
                    }
                    names.push(name.to_string());
                }
                names
            }
            _ => return Err(invalid(index, "assetNames is missing")),
        };
        let selection_revision = match field(object, "selectionRevision") {
            Some(v) => whole_number(v, "selectionRevision")?,
            None => 0,
        };
        let selection_revision = u8::try_from(selection_revision)
            .ok()
            .filter(|r| matches!(r, 1 | 2))
            .ok_or_else(|| invalid(index, "selectionRevision must be 1 or 2"))?;
        let validated_at = timestamp(object, "validatedAt", &owner)?;
        if validated_at > generated_at.plus_secs(CLOCK_SKEW_SECS) {
            return Err(invalid(index, "was validated after the document says it was generated"));
        }
        Ok(Self {
            source,
            repository,
            preferred_release: text(object, "preferredRelease")?.filter(|r| !r.trim().is_empty()),
            release_tag,
            asset_names,
            validated_at,
            selection_revision,
        })
    }
}

impl PlatformDocument {
    /// Read and validate a document.
    pub fn parse(text_in: &str) -> Result<Self, CatalogError> {
        if text_in.len() > MAX_BYTES {
            return Err(CatalogError::shape("the platform metadata is larger than 16 MiB"));
        }
        let root: Value =
            serde_json::from_str(text_in.strip_prefix('\u{feff}').unwrap_or(text_in)).map_err(|e| CatalogError::Json(e.to_string()))?;
        let Value::Object(object) = &root else {
            return Err(CatalogError::wrong_type("the platform metadata", "an object", &root));
        };
        let format_revision = whole_number(field(object, "formatRevision").unwrap_or(&Value::Null), "formatRevision")?;
        if format_revision != 1 {
            return Err(CatalogError::shape(format!("formatRevision {format_revision} is not one this version reads (1)")));
        }
        let generated_at = timestamp(object, "generatedAt", "the document")?;
        let items = match field(object, "entries") {
            Some(Value::Array(items)) => items,
            _ => return Err(CatalogError::shape("entries is missing")),
        };
        if items.len() > MAX_ENTRIES {
            return Err(CatalogError::shape(format!("more than {MAX_ENTRIES} entries")));
        }
        let mut seen = HashSet::new();
        let mut entries = Vec::with_capacity(items.len());
        for (i, item) in items.iter().enumerate() {
            let entry = PlatformEntry::from_value(item, i, generated_at)?;
            if !seen.insert(entry.key()) {
                return Err(invalid(i, format!("repeats {}", entry.repository)));
            }
            entries.push(entry);
        }
        Ok(Self { format_revision: 1, generated_at, entries })
    }

    /// Whether the document claims to come from after `now` (allowing for clocks that differ a little), which a
    /// launcher treats as a reason not to trust a downloaded copy.
    pub fn is_from_the_future(&self, now: Timestamp) -> bool {
        self.generated_at > now.plus_secs(CLOCK_SKEW_SECS)
    }
}

/// Every entry from every document a launcher knows about, the newest check winning where they overlap.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PlatformIndex {
    entries: HashMap<PlatformKey, PlatformEntry>,
}

impl PlatformIndex {
    pub fn from_documents<'a>(documents: impl IntoIterator<Item = &'a PlatformDocument>) -> Self {
        let mut entries: HashMap<PlatformKey, PlatformEntry> = HashMap::new();
        for entry in documents.into_iter().flat_map(|d| &d.entries) {
            match entries.get(&entry.key()) {
                Some(have) if have.validated_at >= entry.validated_at => {}
                _ => {
                    entries.insert(entry.key(), entry.clone());
                }
            }
        }
        Self { entries }
    }

    /// What is known about an app's release, or nothing (which is "unknown", not "unsupported").
    pub fn get(&self, source: RepoSource, repository: &str, preferred_release: Option<&str>) -> Option<&PlatformEntry> {
        self.entries.get(&PlatformKey::new(source, repository, preferred_release))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests;
