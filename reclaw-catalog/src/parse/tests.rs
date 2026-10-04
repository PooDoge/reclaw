use serde_json::json;

use super::*;

fn lenient(value: Value) -> AppEntry {
    parse_app(&value, Mode::Lenient).expect("a readable entry")
}

#[test]
fn a_catalog_entry_reads_with_normalised_tags_and_files() {
    let e = lenient(json!({
        "name": "Super Mario 64", "project": " Ghostship ", "repository": "harbourmasters/ghostship", "folderName": "SuperMario64-Ghostship",
        "appIconUrl": "https://example.com/i.png", "tags": ["Recomp", "N64", "recomp"], "filesToAdd": ["portable.txt", "bad/name"],
        "catalogId": "qcat_1"
    }));
    assert_eq!(e.project.as_deref(), Some(" Ghostship "), "stored as read; trimmed on write");
    assert_eq!(e.tags, ["recomp", "n64"]);
    assert_eq!(e.files_to_add, ["portable.txt"]);
    assert_eq!(e.catalog_id.as_deref(), Some("qcat_1"));
    assert_eq!(e.instance_key(), "github:harbourmasters/ghostship:SuperMario64-Ghostship");
}

#[test]
fn the_icon_comes_from_the_first_key_that_is_not_null_even_when_it_is_empty() {
    assert_eq!(lenient(json!({"appIconUrl": null, "gameIconUrl": "old"})).icon_url.as_deref(), Some("old"));
    assert_eq!(lenient(json!({"appIconUrl": "", "gameIconUrl": "old"})).icon_url.as_deref(), Some(""), "empty wins");
    assert_eq!(lenient(json!({"customDefaultIconUrl": "c"})).icon_url.as_deref(), Some("c"));
    assert_eq!(lenient(json!({})).icon_url, None);
}

#[test]
fn a_manual_app_cannot_be_pinned_updated_or_filtered() {
    let e =
        lenient(json!({"name": "Mine", "folderName": "Mine", "autoUpdate": true, "preferredVersion": "v1", "skippedUpdateVersion": "v2",
        "deferUpdateTracking": true, "releaseAssetFilter": "x", "repositorySource": "gitlab", "installPath": "/games/mine"}));
    assert!(e.is_manual());
    assert!(!e.auto_update && !e.defer_update_tracking);
    assert_eq!((e.preferred_version, e.skipped_update_version, e.release_asset_filter), (None, None, None));
    assert_eq!(e.source, RepoSource::Github);
    assert_eq!(e.install_path.as_deref(), Some("/games/mine"), "the install folder is kept");
}

#[test]
fn a_wrong_type_fails_the_entry_in_both_modes() {
    for value in [json!({"name": 42}), json!({"repository": []}), json!("text"), json!(null)] {
        assert!(parse_app(&value, Mode::Lenient).is_err(), "{value}");
        assert!(parse_app(&value, Mode::Strict).is_err(), "{value}");
    }
}

#[test]
fn collections_and_flags_are_forgiven_only_when_lenient() {
    let odd = json!({"name": "A", "repository": "o/r", "tags": "nope", "filesToAdd": [1, "ok.txt"], "autoUpdate": "yes"});
    let e = lenient(odd.clone());
    assert!(e.tags.is_empty());
    assert_eq!(e.files_to_add, ["ok.txt"]);
    assert!(!e.auto_update);
    assert!(parse_app(&odd, Mode::Strict).is_err());
    assert!(parse_app(&json!({"name": "A", "tags": [1]}), Mode::Strict).is_err());
    assert!(parse_app(&json!({"name": "A", "autoUpdate": null, "deferUpdateTracking": false}), Mode::Strict).is_ok());
}

#[test]
fn all_the_document_shapes_are_read_in_the_order_apps_standard_experimental_custom() {
    let a = |n: &str| json!({"name": n, "repository": format!("o/{n}"), "folderName": n});
    let legacy = json!({"custom": [a("c")], "standard": [a("s")], "apps": [a("p")], "experimental": [a("e")]});
    let names: Vec<_> = parse_apps(&legacy, Mode::Strict).expect("legacy shape").0.into_iter().map(|e| e.name).collect();
    assert_eq!(names, ["p", "s", "e", "c"]);
    assert_eq!(parse_apps(&json!([a("x")]), Mode::Strict).expect("array root").0.len(), 1);
}

#[test]
fn a_document_without_a_list_is_an_empty_catalog_but_not_a_library() {
    assert!(parse_apps(&json!({}), Mode::Lenient).expect("empty catalog").0.is_empty());
    assert!(parse_apps(&json!({}), Mode::Strict).is_err());
    assert!(parse_apps(&json!(null), Mode::Lenient).is_err());
    assert!(parse_apps(&json!({"apps": "x"}), Mode::Lenient).is_err(), "apps that is not a list fails the whole read");
}

#[test]
fn the_first_of_two_entries_for_one_tile_is_kept_and_a_bad_entry_is_skipped_when_lenient() {
    let doc = json!({"apps": [
        {"name": "First", "repository": "o/r", "folderName": "F"},
        {"name": "Second", "repository": "O/R", "folderName": "f"},
        {"name": 7},
        {"name": "Other", "repository": "o/r", "folderName": "G"},
        {"name": "M1", "folderName": ""}, {"name": "M2", "folderName": ""}
    ]});
    let (apps, skipped) = parse_apps(&doc, Mode::Lenient).expect("readable");
    assert_eq!(
        apps.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        ["First", "Other", "M1"],
        "same tile, ignoring case, keeps the first; two empty manual folders collapse"
    );
    assert_eq!(skipped.len(), 1);
    assert_eq!(skipped[0].index, 2);
    let strict = parse_apps(&doc, Mode::Strict).expect_err("the library refuses a bad entry");
    assert!(strict.to_string().contains("app 3"), "{strict}");
}

#[test]
fn the_reclaw_block_is_read_and_a_damaged_one_is_ignored() {
    let ok = lenient(json!({"name": "A", "reclaw": {"summary": "s"}}));
    assert_eq!(ok.extension.and_then(|x| x.summary).as_deref(), Some("s"));
    assert!(lenient(json!({"name": "A", "reclaw": {"videos": 3}})).extension.is_none());
}
