//! Reading app entries from JSON, in the two modes the launcher this format comes from has. A catalog list is read
//! leniently: an entry that cannot be read is skipped and the rest are used. The library is read strictly: it is the
//! user's own data, and a copy with an entry quietly missing must never be written back over the original.
use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::{
    entry::AppEntry,
    error::CatalogError,
    extension::Extension,
    mods::ModsConfig,
    normalize::{self, same_key},
    source::RepoSource,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Lenient,
    Strict,
}

/// An entry that was left out of a lenient read, and why.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Skipped {
    /// Its place in the list, from zero.
    pub index: usize,
    pub reason: String,
}

fn text(object: &Map<String, Value>, key: &str) -> Result<Option<String>, CatalogError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(CatalogError::wrong_type(key, "a string", other)),
    }
}

fn blank_to_none(text: Option<String>) -> Option<String> {
    text.filter(|t| !t.trim().is_empty())
}

/// A list of strings. Anything but strings is an error in strict mode and is skipped otherwise.
fn strings(object: &Map<String, Value>, key: &str, mode: Mode) -> Result<Vec<String>, CatalogError> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => {
            let mut out = Vec::new();
            for item in items {
                match item {
                    Value::String(s) => out.push(s.clone()),
                    other if mode == Mode::Strict => return Err(CatalogError::wrong_type(&format!("an item of {key}"), "a string", other)),
                    _ => {}
                }
            }
            Ok(out)
        }
        Some(other) if mode == Mode::Strict => Err(CatalogError::wrong_type(key, "an array", other)),
        Some(_) => Ok(Vec::new()),
    }
}

/// A true/false flag: only `true` counts as true; strict mode also refuses anything that is not a boolean or null.
fn flag(object: &Map<String, Value>, key: &str, mode: Mode) -> Result<bool, CatalogError> {
    match object.get(key) {
        Some(Value::Bool(b)) => Ok(*b),
        None | Some(Value::Null) => Ok(false),
        Some(other) if mode == Mode::Strict => Err(CatalogError::wrong_type(key, "true or false", other)),
        Some(_) => Ok(false),
    }
}

/// The icon address: `appIconUrl`, else the older `gameIconUrl`, else `customDefaultIconUrl`. A key counts when it is
/// present and not null, so an empty string wins and does not fall through to the next.
fn icon(object: &Map<String, Value>) -> Result<Option<String>, CatalogError> {
    for key in ["appIconUrl", "gameIconUrl", "customDefaultIconUrl"] {
        if matches!(object.get(key), Some(v) if !v.is_null()) {
            return text(object, key);
        }
    }
    Ok(None)
}

/// Read one entry.
pub fn parse_app(value: &Value, mode: Mode) -> Result<AppEntry, CatalogError> {
    let Value::Object(object) = value else {
        return Err(CatalogError::wrong_type("an app entry", "an object", value));
    };
    let repository = text(object, "repository")?.unwrap_or_default();
    let manual = repository.trim().is_empty();
    let mut entry = AppEntry {
        name: text(object, "name")?.unwrap_or_default(),
        project: blank_to_none(text(object, "project")?),
        repository: if manual { String::new() } else { repository },
        source: RepoSource::normalize(text(object, "repositorySource")?.as_deref()),
        folder_name: text(object, "folderName")?.unwrap_or_default(),
        icon_url: icon(object)?,
        tags: normalize::tags(strings(object, "tags", mode)?),
        files_to_add: normalize::files_to_add(strings(object, "filesToAdd", mode)?),
        release_asset_filter: normalize::asset_filter(text(object, "releaseAssetFilter")?.as_deref()),
        mods: object.get("mods").map_or(Ok(ModsConfig::default()), |v| ModsConfig::from_value(v, mode == Mode::Strict))?,
        catalog_id: blank_to_none(text(object, "catalogId")?),
        extension: object.get("reclaw").and_then(|v| Extension::from_value(v).0),
        install_path: text(object, "installPath")?,
        preferred_version: text(object, "preferredVersion")?,
        skipped_update_version: text(object, "skippedUpdateVersion")?,
        custom_display_name: blank_to_none(text(object, "customDisplayName")?),
        auto_update: flag(object, "autoUpdate", mode)?,
        defer_update_tracking: flag(object, "deferUpdateTracking", mode)?,
        linux_runner: blank_to_none(text(object, "linuxRunner")?),
        linux_prefix_path: blank_to_none(text(object, "linuxPrefixPath")?),
        linux_proton_path: blank_to_none(text(object, "linuxProtonPath")?),
        linux_custom_launch_command: blank_to_none(text(object, "linuxCustomLaunchCommand")?),
    };
    if manual {
        // Nothing is downloaded, so nothing can be pinned, skipped, updated or filtered.
        entry.source = RepoSource::Github;
        entry.preferred_version = None;
        entry.skipped_update_version = None;
        entry.auto_update = false;
        entry.defer_update_tracking = false;
        entry.release_asset_filter = None;
    }
    Ok(entry)
}

/// The entries of a document in any of the shapes the format has had: an array; an object with `apps`; or the older
/// object with `standard`, `experimental` and `custom` arrays, read after `apps` in that order.
fn app_values(root: &Value, mode: Mode) -> Result<Vec<&Value>, CatalogError> {
    match root {
        Value::Array(items) => Ok(items.iter().collect()),
        Value::Object(object) => {
            let mut values = Vec::new();
            let mut found = false;
            for key in ["apps", "standard", "experimental", "custom"] {
                match object.get(key) {
                    Some(Value::Array(items)) => {
                        found = true;
                        values.extend(items);
                    }
                    Some(Value::Null) | None => {}
                    Some(other) => return Err(CatalogError::wrong_type(key, "an array", other)),
                }
            }
            if !found && mode == Mode::Strict {
                return Err(CatalogError::shape("the library has no list of apps (expected an \"apps\" array)"));
            }
            Ok(values)
        }
        other => Err(CatalogError::wrong_type("the document", "an object or an array", other)),
    }
}

/// Every entry of a document, repeats (the same library tile, ignoring case) dropped with the first kept. Lenient mode
/// also returns what it had to skip.
pub fn parse_apps(root: &Value, mode: Mode) -> Result<(Vec<AppEntry>, Vec<Skipped>), CatalogError> {
    let mut entries = Vec::new();
    let mut skipped = Vec::new();
    let mut seen = HashSet::new();
    for (index, value) in app_values(root, mode)?.into_iter().enumerate() {
        match parse_app(value, mode) {
            Ok(entry) => {
                if seen.insert(entry.instance_key().to_uppercase()) {
                    entries.push(entry);
                }
            }
            Err(e) if mode == Mode::Lenient => skipped.push(Skipped { index, reason: e.to_string() }),
            Err(e) => return Err(CatalogError::Shape(format!("app {}: {e}", index + 1))),
        }
    }
    debug_assert!(entries.iter().enumerate().all(|(i, a)| entries[..i].iter().all(|b| !same_key(&a.instance_key(), &b.instance_key()))));
    Ok((entries, skipped))
}

#[cfg(test)]
mod tests;
