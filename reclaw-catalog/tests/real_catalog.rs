//! Reads the real community catalog and checks this crate agrees with it. The catalog is somebody else's data and is
//! not copied into this repository: point `QUIVER_CATALOG_DIR` at a checkout of it to run these (they say so and pass
//! when it is not set). The synthetic cases that always run live beside the code.
use std::{fs, path::PathBuf};

use reclaw_catalog::{
    AppEntry, CatalogList, CommunityIndex, PlatformDocument, PlatformIndex, RepoSource, library_to_string, normalize,
    parse::{Mode, parse_app},
    parse_library, parse_list,
    write::{catalog_value, library_value},
};
use serde_json::Value;

fn catalog_dir() -> Option<PathBuf> {
    match std::env::var_os("QUIVER_CATALOG_DIR") {
        Some(dir) => Some(PathBuf::from(dir)),
        None => {
            eprintln!("QUIVER_CATALOG_DIR is not set: skipping the real-catalog check");
            None
        }
    }
}

fn read(dir: &std::path::Path, name: &str) -> String {
    fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("cannot read {name} in {}: {e}", dir.display()))
}

fn list_files(dir: &std::path::Path) -> Vec<(String, String)> {
    let mut files: Vec<_> = fs::read_dir(dir.join("community-app-catalog"))
        .expect("the catalog has a community-app-catalog folder")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            (p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), fs::read_to_string(&p).expect("a readable list"))
        })
        .collect();
    files.sort();
    files
}

fn keys(value: &Value) -> Vec<&str> {
    value.as_object().map(|o| o.keys().map(String::as_str).collect()).unwrap_or_default()
}

#[test]
fn the_index_reads_and_points_at_every_list_file() {
    let Some(dir) = catalog_dir() else { return };
    let index = CommunityIndex::parse(&read(&dir, "index.json")).expect("the real index reads");
    let sources = index.sources();
    assert_eq!(sources.len(), index.lists.len(), "every list has an address");
    assert!(index.platform_metadata_url.as_deref().is_some_and(|u| u.starts_with("https://")));
    let files = list_files(&dir);
    for source in &sources {
        let file = source.url.rsplit('/').next().unwrap_or_default();
        assert!(files.iter().any(|(name, _)| name == file), "{} is in the catalog folder", source.url);
        assert_eq!(source.cache_stem(), source.id, "real ids are already safe file names");
    }
}

#[test]
fn every_list_reads_with_nothing_skipped_dropped_or_renamed() {
    let Some(dir) = catalog_dir() else { return };
    for (name, text) in list_files(&dir) {
        let list: CatalogList = parse_list(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
        let raw: Value = serde_json::from_str(&text).expect("json");
        let raw_apps = raw["apps"].as_array().unwrap_or_else(|| panic!("{name} has an apps array"));
        assert!(list.skipped.is_empty(), "{name} skipped {:?}", list.skipped);
        assert_eq!(list.apps.len(), raw_apps.len(), "{name}: no entry was merged or dropped");
        assert!(
            list.name.is_some() && !list.version.is_empty() && list.version.len() < 32,
            "{name}: the author's own version is used, not a hash"
        );
        assert!(list.icon_url.is_some() || raw.get("iconUrl").is_none(), "{name}: an icon the author gave is kept");
        eprintln!("{name}: {} apps, version {}", list.apps.len(), list.version);
    }
}

/// The entry as the format would have it if its author had followed the normalisation rules: tags and file names
/// trimmed and without repeats. The real catalog has a few hand-edited entries that do not.
fn canonical(original: &Value) -> Value {
    let mut value = original.clone();
    for key in ["tags", "filesToAdd"] {
        if let Some(Value::Array(items)) = original.get(key) {
            let given = items.iter().filter_map(Value::as_str);
            let cleaned = if key == "tags" { normalize::tags(given) } else { normalize::files_to_add(given) };
            value[key] = Value::Array(cleaned.into_iter().map(Value::String).collect());
        }
    }
    value
}

#[test]
fn every_entry_writes_back_as_the_same_json() {
    let Some(dir) = catalog_dir() else { return };
    let (mut checked, mut reordered, mut uncanonical) = (0, Vec::new(), Vec::new());
    for (name, text) in list_files(&dir) {
        let raw: Value = serde_json::from_str(&text).expect("json");
        for original in raw["apps"].as_array().expect("apps") {
            let entry = parse_app(original, Mode::Lenient).expect("an entry reads");
            let written = catalog_value(&entry);
            let expected = canonical(original);
            assert_eq!(written, expected, "{name}: {} did not survive reading and writing", entry.name);
            if &expected != original {
                uncanonical.push(format!("{name}: {}", entry.name));
            }
            if keys(&written) != keys(original) {
                reordered.push(format!("{name}: {} {:?} vs {:?}", entry.name, keys(&written), keys(original)));
            }
            checked += 1;
        }
    }
    assert!(checked > 0);
    eprintln!("{checked} entries checked; {} have tags or files the format would tidy: {uncanonical:?}", uncanonical.len());
    // Hand-edited catalogs put `releaseAssetFilter` in two different places (6 entries before `folderName`, 2 after the
    // icon), so key order is style and not part of the format: every reader here and in Quiver finds keys by name.
    // Ours matches the majority; this only reports the difference.
    eprintln!("{} entries list their keys in another order than we write them", reordered.len());
}

#[test]
fn the_whole_catalog_is_a_valid_library_and_survives_a_round_trip() {
    let Some(dir) = catalog_dir() else { return };
    let all: Vec<AppEntry> = list_files(&dir).iter().flat_map(|(_, text)| parse_list(text).expect("a list").apps).collect();
    let text = library_to_string(&all);
    let back = parse_library(&text).expect("what we write, we can read strictly");
    // A library is one app per tile: the lists overlap (one repository in two lists), and reading it collapses those.
    let mut keys: Vec<String> = all.iter().map(|a| a.instance_key().to_uppercase()).collect();
    keys.sort();
    keys.dedup();
    assert_eq!(back.len(), keys.len());
    for entry in &back {
        assert_eq!(parse_library(&library_to_string(std::slice::from_ref(entry))), Ok(vec![entry.clone()]));
        let value = library_value(entry);
        for always in ["name", "folderName", "installPath", "appIconUrl"] {
            assert!(value.get(always).is_some(), "{} lacks {always}", entry.name);
        }
    }
}

#[test]
fn the_platform_index_validates_and_covers_the_catalog() {
    let Some(dir) = catalog_dir() else { return };
    let document = PlatformDocument::parse(&read(&dir, "platform-index.json")).expect("the real platform index is valid");
    assert!(!document.entries.is_empty());
    let index = PlatformIndex::from_documents([&document]);
    assert_eq!(index.len(), document.entries.len(), "no two entries share a key");
    let apps: Vec<AppEntry> =
        list_files(&dir).iter().flat_map(|(_, text)| parse_list(text).expect("a list").apps).filter(|a| !a.is_manual()).collect();
    let known = apps.iter().filter(|a| index.get(a.source, &a.repository, a.preferred_version.as_deref()).is_some()).count();
    eprintln!("{known} of {} hosted apps have platform metadata", apps.len());
    assert!(known * 10 >= apps.len() * 9, "the index should cover nearly every catalog repository ({known} of {})", apps.len());
    assert!(apps.iter().any(|a| a.source == RepoSource::Gitlab), "the catalog has GitLab apps, and they resolve");
}

#[test]
fn every_app_in_the_real_catalog_has_a_system() {
    let Some(dir) = catalog_dir() else { return };
    let mut counts = std::collections::BTreeMap::new();
    let mut unknown = Vec::new();
    for (name, text) in list_files(&dir) {
        for app in parse_list(&text).expect("a list").apps {
            let platform = app.system();
            if platform.is_known() {
                *counts.entry((name.clone(), platform.label())).or_insert(0) += 1;
            } else {
                unknown.push(format!("{name}: {} {:?}", app.name, app.tags));
            }
        }
    }
    eprintln!("{counts:#?}");
    assert!(unknown.is_empty(), "apps no tag places on a system: {unknown:#?}");
}
