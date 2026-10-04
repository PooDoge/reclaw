use super::*;

const GENERATED: &str = "2026-10-04T12:29:25.4869655+00:00";
const CHECKED: &str = "2026-10-04T12:28:25.2762956+00:00";

fn entry(provider: &str, repo: &str, tag: &str, assets: &str, checked: &str, revision: &str) -> String {
    format!(
        r#"{{"provider": "{provider}", "repository": "{repo}", "preferredRelease": null, "releaseTag": "{tag}", "assetNames": [{assets}], "validatedAt": "{checked}", "selectionRevision": {revision}}}"#
    )
}

fn document(generated: &str, entries: &[String]) -> String {
    format!(r#"{{"formatRevision": 1, "generatedAt": "{generated}", "entries": [{}]}}"#, entries.join(","))
}

fn good() -> String {
    document(GENERATED, &[entry("github", "999sian/melee-pc", "v1", r#""Melee-Windows.zip", "melee-linux.tar.gz""#, CHECKED, "1")])
}

#[test]
fn a_document_as_published_reads_and_answers_lookups_ignoring_github_case() {
    let doc = PlatformDocument::parse(&good()).expect("valid");
    assert_eq!(doc.entries.len(), 1);
    let index = PlatformIndex::from_documents([&doc]);
    for repo in ["999sian/melee-pc", "999SIAN/Melee-PC", " 999sian/melee-pc "] {
        assert!(index.get(RepoSource::Github, repo, None).is_some(), "{repo}");
    }
    assert!(index.get(RepoSource::Gitlab, "999sian/melee-pc", None).is_none(), "another service is another repository");
    assert!(index.get(RepoSource::Github, "999sian/melee-pc", Some("v0.1")).is_none(), "a pinned release asks about its own result");
}

#[test]
fn gitlab_repositories_are_matched_exactly() {
    let text = document(GENERATED, &[entry("gitlab", "Group/Project", "v1", "", CHECKED, "1")]);
    let index = PlatformIndex::from_documents([&PlatformDocument::parse(&text).expect("valid")]);
    assert!(index.get(RepoSource::Gitlab, "Group/Project", None).is_some());
    assert!(index.get(RepoSource::Gitlab, "group/project", None).is_none());
}

#[test]
fn one_bad_entry_rejects_the_whole_document() {
    let ok = entry("github", "a/b", "v1", "", CHECKED, "1");
    let cases = [
        entry("GitHub", "c/d", "v1", "", CHECKED, "1"),
        entry("bitbucket", "c/d", "v1", "", CHECKED, "1"),
        entry("github", "  ", "v1", "", CHECKED, "1"),
        entry("github", "c/d", "v1", "", CHECKED, "3"),
        entry("github", "c/d", "v1", "", CHECKED, "0"),
        entry("github", "c/d", "v1", "", "2026-10-04T12:35:00+00:00", "1"),
        entry("github", "c/d", "v1", "", "not a date", "1"),
        entry("github", "c/d", "v1", "", "0001-01-01T00:00:00+00:00", "1"),
        entry("github", "A/B", "v1", "", CHECKED, "1"),
        r#"{"provider": "github", "repository": "c/d", "assetNames": [], "validatedAt": "2026-10-04T12:00:00Z", "selectionRevision": 1}"#.into(),
        r#"{"provider": "github", "repository": "c/d", "releaseTag": "v", "assetNames": [5], "validatedAt": "2026-10-04T12:00:00Z", "selectionRevision": 1}"#.into(),
        r#"{"provider": "github", "repository": "c/d", "releaseTag": "v", "validatedAt": "2026-10-04T12:00:00Z", "selectionRevision": 1}"#.into(),
        "42".into(),
    ];
    for bad in cases {
        let text = document(GENERATED, &[ok.clone(), bad.clone()]);
        assert!(PlatformDocument::parse(&text).is_err(), "should reject: {bad}");
    }
}

#[test]
fn an_empty_tag_with_no_assets_is_a_valid_nothing_usable_result() {
    let text = document(GENERATED, &[entry("github", "a/b", "", "", CHECKED, "1")]);
    let doc = PlatformDocument::parse(&text).expect("valid");
    assert!(doc.entries[0].release_tag.is_empty() && doc.entries[0].asset_names.is_empty());
}

#[test]
fn the_document_header_is_checked() {
    for bad in [
        r#"{"formatRevision": 2, "generatedAt": "2026-10-04T12:29:25Z", "entries": []}"#,
        r#"{"generatedAt": "2026-10-04T12:29:25Z", "entries": []}"#,
        r#"{"formatRevision": 1, "entries": []}"#,
        r#"{"formatRevision": 1, "generatedAt": "0001-01-01T00:00:00Z", "entries": []}"#,
        r#"{"formatRevision": 1, "generatedAt": "2026-10-04T12:29:25Z"}"#,
        r#"{"formatRevision": 1, "generatedAt": "2026-10-04T12:29:25"}"#,
        "[]",
        "",
        "{",
    ] {
        assert!(PlatformDocument::parse(bad).is_err(), "{bad:?}");
    }
    // Web defaults let a number arrive as text, and keys in any case.
    assert!(PlatformDocument::parse(r#"{"FormatRevision": "1", "GeneratedAt": "2026-10-04T12:29:25Z", "Entries": []}"#).is_ok());
}

#[test]
fn limits_hold() {
    let too_long = "x".repeat(2049);
    let text = document(GENERATED, &[entry("github", "a/b", "v", &format!("\"{too_long}\""), CHECKED, "1")]);
    assert!(PlatformDocument::parse(&text).is_err());
    let at_limit = "x".repeat(2048);
    let text = document(GENERATED, &[entry("github", "a/b", "v", &format!("\"{at_limit}\""), CHECKED, "1")]);
    assert!(PlatformDocument::parse(&text).is_ok());
    let many: Vec<String> = (0..10_001).map(|i| entry("github", &format!("o/r{i}"), "v", "", CHECKED, "1")).collect();
    assert!(PlatformDocument::parse(&document(GENERATED, &many)).is_err(), "more than 10000 entries");
}

#[test]
fn a_document_from_the_future_is_noticed_with_five_minutes_of_grace() {
    let doc = PlatformDocument::parse(&good()).expect("valid");
    let at = |s: &str| Timestamp::parse(s).expect("a date");
    assert!(!doc.is_from_the_future(at("2026-10-04T12:29:25Z")));
    assert!(!doc.is_from_the_future(at("2026-10-04T12:25:00Z")), "inside the grace");
    assert!(doc.is_from_the_future(at("2026-10-04T12:20:00Z")));
}

#[test]
fn freshness_needs_revision_two_and_under_a_day() {
    let doc_text = document(GENERATED, &[entry("github", "a/one", "v", "", CHECKED, "1"), entry("github", "a/two", "v", "", CHECKED, "2")]);
    let doc = PlatformDocument::parse(&doc_text).expect("valid");
    let now = Timestamp::parse("2026-10-04T13:00:00Z").expect("a date");
    assert!(!doc.entries[0].is_fresh(now), "revision 1 is usable but never fresh");
    assert!(doc.entries[1].is_fresh(now));
    assert!(!doc.entries[1].is_fresh(now.plus_secs(24 * 3600)));
}

#[test]
fn where_documents_overlap_the_newest_check_wins() {
    let older =
        PlatformDocument::parse(&document(GENERATED, &[entry("github", "a/b", "old", "", "2026-10-04T12:00:00Z", "1")])).expect("valid");
    let newer =
        PlatformDocument::parse(&document(GENERATED, &[entry("github", "a/b", "new", "", "2026-10-04T12:20:00Z", "1")])).expect("valid");
    for docs in [[&older, &newer], [&newer, &older]] {
        let index = PlatformIndex::from_documents(docs);
        assert_eq!(index.len(), 1);
        assert_eq!(index.get(RepoSource::Github, "a/b", None).map(|e| e.release_tag.as_str()), Some("new"));
    }
}
