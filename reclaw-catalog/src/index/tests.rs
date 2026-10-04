use super::*;

const REAL_SHAPE: &str = r#"{
  "version": 2,
  "lists": [
    {"id": "f139f3ee-496e-4457-8c86-17371b752948", "remoteLocation": "https://example.test/Nintendo.json"},
    {"id": "5d8e0593", "remoteLocation": "https://example.test/PlayStation.json"}
  ],
  "platformMetadataUrl": "https://example.test/platform-index.json"
}"#;

#[test]
fn the_index_as_published_reads() {
    let index = CommunityIndex::parse(REAL_SHAPE).expect("valid");
    assert_eq!(index.version, 2);
    assert_eq!(index.lists.len(), 2);
    assert_eq!(index.platform_metadata_url.as_deref(), Some("https://example.test/platform-index.json"));
    let sources = index.sources();
    assert_eq!(sources[0].url, "https://example.test/Nintendo.json");
    assert_eq!(sources[0].name, "", "the name lives in the list file");
}

#[test]
fn keys_match_ignoring_case_but_the_version_must_be_a_number() {
    let loud = CommunityIndex::parse(r#"{"VERSION": 3, "Lists": [{"ID": "a", "RemoteLocation": "https://e.test/a.json"}]}"#)
        .expect("case-insensitive");
    assert_eq!((loud.version, loud.lists[0].id.as_str()), (3, "a"));
    for bad in
        [r#"{"version": "2", "lists": [{"id": "a", "remoteLocation": "https://e.test/a"}]}"#, r#"{"version": 2.5, "lists": [{"id": "a"}]}"#]
    {
        assert!(CommunityIndex::parse(bad).is_err(), "{bad}");
    }
}

#[test]
fn an_index_without_lists_or_with_the_wrong_shape_is_refused() {
    for bad in ["", "[]", "{}", r#"{"version": 2, "lists": []}"#, r#"{"lists": "x"}"#, r#"{"lists": [1]}"#, r#"{"lists": [{"id": 5}]}"#] {
        assert!(CommunityIndex::parse(bad).is_err(), "{bad:?}");
    }
}

#[test]
fn an_address_is_the_remote_location_else_a_web_location_else_nothing() {
    let list = |remote: Option<&str>, location: &str| IndexList {
        remote_location: remote.map(str::to_string),
        location: location.into(),
        ..Default::default()
    };
    assert_eq!(list(Some(" https://a.test/x.json "), "bundled/x.json").address().as_deref(), Some("https://a.test/x.json"));
    assert_eq!(list(None, " HTTPS://a.test/y.json").address().as_deref(), Some("HTTPS://a.test/y.json"));
    assert_eq!(list(Some("  "), "https://a.test/z.json").address().as_deref(), Some("https://a.test/z.json"));
    assert_eq!(list(None, "bundled/x.json").address(), None);
    assert_eq!(list(None, "").address(), None);
}

#[test]
fn lists_without_an_address_and_repeated_ids_are_left_out() {
    let index = CommunityIndex::parse(
        r#"{"version": 2, "lists": [
          {"id": "a", "remoteLocation": "https://e.test/a.json"},
          {"id": " A ", "remoteLocation": "https://e.test/again.json"},
          {"id": "b", "location": "local.json"},
          {"id": "c", "name": " C ", "location": "https://e.test/c.json"}]}"#,
    )
    .expect("valid");
    let ids: Vec<_> = index.sources().into_iter().map(|s| (s.id, s.name)).collect();
    assert_eq!(ids, [("a".to_string(), String::new()), ("c".to_string(), "C".to_string())]);
}

#[test]
fn an_id_becomes_a_safe_file_name_and_different_ids_never_share_one() {
    let stem = |id: &str| IndexSource { id: id.into(), name: String::new(), description: String::new(), url: String::new() }.cache_stem();
    assert_eq!(stem("f139f3ee-496e-4457-8c86-17371b752948"), "f139f3ee-496e-4457-8c86-17371b752948");
    for hostile in ["../../etc/passwd", "a/b", "a\\b", "..", ".", "", "con:", "a b"] {
        let s = stem(hostile);
        assert!(!s.is_empty() && s != "." && s != ".." && !s.contains(['/', '\\', ':', ' ']), "{hostile:?} -> {s:?}");
    }
    assert_ne!(stem("a/b"), stem("a_b"));
    assert_eq!(stem("a/b"), stem("a/b"), "stable");
}
