use serde_json::json;

use super::*;
use crate::{
    mods::{ModLayout, ModSource, ModsConfig},
    parse::{Mode, parse_app},
};

fn keys(value: &Value) -> Vec<String> {
    value.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
}

fn hosted() -> AppEntry {
    AppEntry {
        name: "Super Mario 64".into(),
        project: Some("  Ghostship ".into()),
        repository: "harbourmasters/ghostship".into(),
        folder_name: "SuperMario64-Ghostship".into(),
        icon_url: Some("https://example.com/i.png".into()),
        tags: vec!["recomp".into(), "n64".into()],
        ..Default::default()
    }
}

#[test]
fn a_hosted_library_entry_writes_the_four_always_keys_and_the_hosted_ones_in_quivers_order() {
    let value = library_value(&hosted());
    assert_eq!(
        keys(&value),
        ["name", "folderName", "installPath", "appIconUrl", "repository", "preferredVersion", "skippedUpdateVersion", "project", "tags"]
    );
    assert_eq!(value["installPath"], Value::Null);
    assert_eq!(value["preferredVersion"], Value::Null, "explicit nulls");
    assert_eq!(value["project"], json!("Ghostship"), "trimmed on write");
}

#[test]
fn every_optional_key_appears_in_order_when_set() {
    let e = AppEntry {
        install_path: Some("/g".into()),
        preferred_version: Some("v1".into()),
        skipped_update_version: Some("v2".into()),
        custom_display_name: Some("Mario".into()),
        source: RepoSource::Gitlab,
        auto_update: true,
        defer_update_tracking: true,
        linux_runner: Some("wine".into()),
        linux_prefix_path: Some("/p".into()),
        linux_proton_path: Some("/pr".into()),
        linux_custom_launch_command: Some("{exe}".into()),
        files_to_add: vec!["portable.txt".into()],
        release_asset_filter: Some("Mario".into()),
        mods: ModsConfig {
            path: "mods".into(),
            layout: ModLayout::FolderPerMod,
            sources: vec![ModSource { provider: "thunderstore".into(), source_url: "u".into() }],
        },
        catalog_id: Some("qcat_1".into()),
        catalog_entry_id: Some("k17abc".into()),
        catalog_snapshot: Some(crate::snapshot::CatalogSnapshot { name: Some("Super Mario 64".into()), ..Default::default() }),
        ..hosted()
    };
    assert_eq!(
        keys(&library_value(&e)),
        [
            "name",
            "folderName",
            "installPath",
            "appIconUrl",
            "repository",
            "preferredVersion",
            "skippedUpdateVersion",
            "project",
            "customDisplayName",
            "repositorySource",
            "autoUpdate",
            "deferUpdateTracking",
            "linuxRunner",
            "linuxPrefixPath",
            "linuxProtonPath",
            "linuxCustomLaunchCommand",
            "tags",
            "catalogEntryId",
            "catalog",
            "filesToAdd",
            "releaseAssetFilter",
            "mods",
            "catalogId"
        ]
    );
}

#[test]
fn the_default_runner_and_a_manual_apps_missing_hosting_are_not_written() {
    let manual = AppEntry {
        name: "Mine".into(),
        folder_name: "Mine".into(),
        linux_runner: Some("Auto".into()),
        auto_update: true,
        ..Default::default()
    };
    assert_eq!(keys(&library_value(&manual)), ["name", "folderName", "installPath", "appIconUrl"]);
}

#[test]
fn a_catalog_entry_leaves_out_what_belongs_to_one_user_and_follows_the_authors_order() {
    let e = AppEntry {
        install_path: Some("/g".into()),
        auto_update: true,
        source: RepoSource::Gitlab,
        release_asset_filter: Some("Mario".into()),
        files_to_add: vec!["portable.txt".into()],
        ..hosted()
    };
    assert_eq!(
        keys(&catalog_value(&e)),
        ["name", "project", "repository", "repositorySource", "releaseAssetFilter", "folderName", "appIconUrl", "tags", "filesToAdd"]
    );
}

#[test]
fn what_is_written_reads_back_as_the_same_entry_in_both_projections() {
    let e = AppEntry {
        preferred_version: Some("v1".into()),
        auto_update: true,
        custom_display_name: Some("Mario".into()),
        project: Some("Ghostship".into()),
        ..hosted()
    };
    assert_eq!(parse_app(&library_value(&e), Mode::Strict), Ok(e.clone()));
    let for_catalog = AppEntry { preferred_version: None, auto_update: false, custom_display_name: None, ..e };
    assert_eq!(parse_app(&catalog_value(&for_catalog), Mode::Lenient), Ok(for_catalog));
}

#[test]
fn the_library_text_is_an_apps_object_with_two_space_indentation() {
    let text = library_to_string(&[hosted()]);
    assert!(text.starts_with("{\n  \"apps\": [\n    {\n      \"name\": \"Super Mario 64\""), "{text}");
    assert_eq!(library_to_string(&[]), "{\n  \"apps\": []\n}");
}

#[test]
fn a_library_quiver_3_5_wrote_keeps_its_link_to_the_site_when_reclaw_saves_it() {
    let written = json!({
        "name": "Super Mario 64", "folderName": "SM64", "repository": "o/r",
        "catalogEntryId": "k17abc",
        "catalog": {"name": "Super Mario 64", "project": "Ghostship", "appIconUrl": null, "tags": ["n64"]}
    });
    let entry = parse_app(&written, Mode::Strict).expect("a valid library entry");
    assert_eq!(entry.catalog_entry_id.as_deref(), Some("k17abc"));
    let again = library_value(&entry);
    assert_eq!(again["catalogEntryId"], json!("k17abc"));
    assert_eq!(again["catalog"], written["catalog"], "the block goes back as it came");
    assert_eq!(keys(&catalog_value(&entry)).iter().filter(|k| k.starts_with("catalog")).count(), 0, "a catalog entry has neither");
}
