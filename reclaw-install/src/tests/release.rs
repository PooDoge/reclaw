use serde_json::json;

use super::release_with;
use crate::release::{parse_github_release, parse_gitlab_release, parse_list, select_release};

#[test]
fn a_github_release_keeps_what_the_installer_needs() {
    let value = json!({
        "tag_name": "v1.4.2", "name": "Spring", "prerelease": false, "draft": false, "body": "Fixes.", "published_at": "2026-05-01T10:00:00Z",
        "html_url": "https://github.com/o/r/releases/tag/v1.4.2",
        "assets": [
            {"name": "Game-linux.tar.gz", "browser_download_url": "https://github.com/o/r/releases/download/v1.4.2/Game-linux.tar.gz", "size": 1234,
             "digest": "sha256:ABCDEF"},
            {"name": "", "browser_download_url": "https://x.test/nameless"},
            {"name": "no-url"}
        ]
    });
    let release = parse_github_release(&value).expect("a release");
    assert_eq!((release.tag.as_str(), release.name.as_str(), release.prerelease), ("v1.4.2", "Spring", false));
    assert_eq!(release.assets.len(), 1, "a file with no name or no address is dropped");
    assert_eq!(release.assets[0].size, Some(1234));
    assert_eq!(release.assets[0].sha256.as_deref(), Some("abcdef"));
    assert_eq!(release.notes, "Fixes.");
}

#[test]
fn drafts_and_tagless_entries_are_not_releases() {
    assert!(parse_github_release(&json!({"tag_name": "v1", "draft": true})).is_none());
    assert!(parse_github_release(&json!({"name": "no tag"})).is_none());
    assert!(parse_github_release(&json!("a string")).is_none());
}

#[test]
fn gitlab_links_become_files_only_when_they_are_on_gitlab() {
    let value = json!({
        "tag_name": "v2", "upcoming_release": true, "description": "Notes",
        "assets": {
            "count": 4,
            "sources": [{"format": "zip", "url": "https://gitlab.com/o/r/-/archive/v2/r-v2.zip"}],
            "links": [
                {"name": "Game-Linux", "url": "https://gitlab.com/o/r/-/packages/1", "direct_asset_url": "https://gitlab.com/o/r/-/releases/v2/downloads/game-linux"},
                {"name": "Game-Windows.zip", "url": "https://gitlab.com/o/r/-/jobs/9/artifacts/download"},
                {"name": "Elsewhere.zip", "url": "https://evil.example.com/x.zip"},
                {"name": "Sneaky.zip", "url": "https://gitlab.com.evil.example.com/x.zip"},
                {"name": "Userinfo.zip", "url": "https://gitlab.com@evil.example.com/x.zip"}
            ]
        }
    });
    let release = parse_gitlab_release(&value).expect("a release");
    assert!(release.prerelease, "an upcoming release is a pre-release");
    let names: Vec<_> = release.assets.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Game-Linux", "Game-Windows.zip"]);
    assert_eq!(release.assets[0].url, "https://gitlab.com/o/r/-/releases/v2/downloads/game-linux", "the direct link wins");
}

#[test]
fn a_list_and_a_single_release_both_parse_and_garbage_does_not() {
    let list = br#"[{"tag_name":"v2","assets":[]},{"nope":1},{"tag_name":"v1","assets":[]}]"#;
    assert_eq!(parse_list(list, parse_github_release).expect("a list").len(), 2);
    let one = br#"{"tag_name":"v3","assets":[]}"#;
    assert_eq!(parse_list(one, parse_github_release).expect("one").len(), 1);
    assert!(parse_list(b"<html>rate limited</html>", parse_github_release).is_err());
    assert!(parse_list(b"3", parse_github_release).is_err());
}

/// The tag of the release picked, or nothing.
fn tags(_releases: &[crate::Release], pick: Option<&crate::Release>) -> String {
    pick.map(|r| r.tag.clone()).unwrap_or_default()
}

#[test]
fn the_newest_stable_release_with_files_is_current() {
    let releases = vec![
        release_with("v3-beta", true, &["g-linux.tar.gz"]),
        release_with("v2-empty", false, &["notes.json"]),
        release_with("v2", false, &["g-linux.tar.gz"]),
        release_with("v1", false, &["g-linux.tar.gz"]),
    ];
    assert_eq!(tags(&releases, select_release(&releases, None, None, false)), "v2");
}

#[test]
fn a_pin_beats_everything_and_the_hosts_latest_beats_the_list_order() {
    let releases = vec![
        release_with("v3", false, &["g-linux.tar.gz"]),
        release_with("v2", false, &["g-linux.tar.gz"]),
        release_with("v1", false, &["g-linux.tar.gz"]),
    ];
    assert_eq!(tags(&releases, select_release(&releases, Some("1"), Some("v2"), false)), "v1", "a pin matches v1 and 1 alike");
    assert_eq!(tags(&releases, select_release(&releases, None, Some("v2"), false)), "v2");
    assert_eq!(tags(&releases, select_release(&releases, Some("v9"), Some("v2"), false)), "v2", "a pin that does not exist is ignored");
}

#[test]
fn prereleases_are_used_when_asked_for_or_when_they_are_all_there_is() {
    let releases = vec![release_with("v3-beta", true, &["g-linux.tar.gz"]), release_with("v2", false, &["g-linux.tar.gz"])];
    assert_eq!(tags(&releases, select_release(&releases, None, None, false)), "v2");
    assert_eq!(tags(&releases, select_release(&releases, None, None, true)), "v3-beta");
    let only_beta = vec![release_with("v1-beta", true, &["g-linux.tar.gz"])];
    assert_eq!(tags(&only_beta, select_release(&only_beta, None, None, false)), "v1-beta");
}

#[test]
fn no_releases_with_files_is_no_release() {
    let releases = vec![release_with("v1", false, &["notes.json", "g.sha256"])];
    assert!(select_release(&releases, None, None, true).is_none());
    assert!(select_release(&[], None, None, false).is_none());
}
