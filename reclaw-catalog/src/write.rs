//! Writing app entries. Two projections of the same entry, each in the key order its readers are used to: the library
//! as Quiver saves it, and a catalog entry as authors write it (without the fields that belong to one user).
use serde_json::{Map, Value};

use crate::{entry::AppEntry, source::RepoSource};

fn put(object: &mut Map<String, Value>, key: &str, value: impl Into<Value>) {
    object.insert(key.to_string(), value.into());
}

fn optional_text(text: &Option<String>) -> Value {
    text.as_ref().map_or(Value::Null, |t| Value::String(t.clone()))
}

fn put_trimmed(object: &mut Map<String, Value>, key: &str, text: &Option<String>) {
    if let Some(t) = text.as_deref().map(str::trim).filter(|t| !t.is_empty()) {
        put(object, key, t);
    }
}

fn put_list(object: &mut Map<String, Value>, key: &str, items: &[String]) {
    if !items.is_empty() {
        put(object, key, items.iter().cloned().map(Value::String).collect::<Vec<_>>());
    }
}

/// The tail every projection shares: Reclaw's two additions, last, where Quiver's readers never look.
fn put_additions(object: &mut Map<String, Value>, entry: &AppEntry) {
    put_trimmed(object, "catalogId", &entry.catalog_id);
    if let Some(extension) = &entry.extension {
        put(object, "reclaw", extension.to_value());
    }
}

/// An entry as the library file keeps it. Defaults are left out (`autoUpdate` only when it is on), but the four
/// fields Quiver always writes are written even when empty, as explicit nulls.
pub fn library_value(e: &AppEntry) -> Value {
    let hosted = !e.is_manual();
    let mut o = Map::new();
    put(&mut o, "name", e.name.clone());
    put(&mut o, "folderName", e.folder_name.clone());
    put(&mut o, "installPath", optional_text(&e.install_path));
    put(&mut o, "appIconUrl", optional_text(&e.icon_url));
    if hosted {
        put(&mut o, "repository", e.repository.clone());
        put(&mut o, "preferredVersion", optional_text(&e.preferred_version));
        put(&mut o, "skippedUpdateVersion", optional_text(&e.skipped_update_version));
    }
    put_trimmed(&mut o, "project", &e.project);
    put_trimmed(&mut o, "customDisplayName", &e.custom_display_name);
    if hosted && e.source == RepoSource::Gitlab {
        put(&mut o, "repositorySource", "gitlab");
    }
    if hosted && e.auto_update {
        put(&mut o, "autoUpdate", true);
    }
    if hosted && e.defer_update_tracking {
        put(&mut o, "deferUpdateTracking", true);
    }
    // "auto" is the default runner and is not written.
    if let Some(runner) = e.linux_runner.as_deref().filter(|r| !r.trim().is_empty() && !r.trim().eq_ignore_ascii_case("auto")) {
        put(&mut o, "linuxRunner", runner);
    }
    for (key, value) in [
        ("linuxPrefixPath", &e.linux_prefix_path),
        ("linuxProtonPath", &e.linux_proton_path),
        ("linuxCustomLaunchCommand", &e.linux_custom_launch_command),
    ] {
        if value.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            put(&mut o, key, optional_text(value));
        }
    }
    put_list(&mut o, "tags", &e.tags);
    put_list(&mut o, "filesToAdd", &e.files_to_add);
    if hosted {
        put_trimmed(&mut o, "releaseAssetFilter", &e.release_asset_filter);
    }
    if !e.mods.is_empty() {
        put(&mut o, "mods", e.mods.to_value());
    }
    put_additions(&mut o, e);
    Value::Object(o)
}

/// An entry as a catalog author writes it: what the app is, nothing about where this user put it or what they chose.
pub fn catalog_value(e: &AppEntry) -> Value {
    let hosted = !e.is_manual();
    let mut o = Map::new();
    put(&mut o, "name", e.name.clone());
    put_trimmed(&mut o, "project", &e.project);
    if hosted {
        put(&mut o, "repository", e.repository.clone());
        if e.source == RepoSource::Gitlab {
            put(&mut o, "repositorySource", "gitlab");
        }
        put_trimmed(&mut o, "releaseAssetFilter", &e.release_asset_filter);
    }
    put(&mut o, "folderName", e.folder_name.clone());
    if let Some(icon) = &e.icon_url {
        put(&mut o, "appIconUrl", icon.clone());
    }
    put_list(&mut o, "tags", &e.tags);
    put_list(&mut o, "filesToAdd", &e.files_to_add);
    if !e.mods.is_empty() {
        put(&mut o, "mods", e.mods.to_value());
    }
    put_additions(&mut o, e);
    Value::Object(o)
}

/// The library file's text: `{"apps": [...]}`, two-space indented.
pub fn library_to_string(apps: &[AppEntry]) -> String {
    let mut root = Map::new();
    put(&mut root, "apps", apps.iter().map(library_value).collect::<Vec<_>>());
    // A map of strings to JSON values always serialises.
    serde_json::to_string_pretty(&Value::Object(root)).unwrap_or_else(|_| "{\n  \"apps\": []\n}".to_string())
}

#[cfg(test)]
mod tests;
