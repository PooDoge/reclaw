use super::*;

fn app(repo: &str, folder: &str) -> AppEntry {
    AppEntry { name: "G".into(), repository: repo.into(), folder_name: folder.into(), ..Default::default() }
}

#[test]
fn a_full_list_reads_with_its_metadata() {
    let list = parse_list(
        r#"{"name": " Nintendo ", "description": "Ports", "version": "1.0.19", "iconUrl": "https://x.test/i.png",
            "preferredTagFilters": ["Recomp", "N64", "recomp"], "hiddenTagFilters": ["decompilation"],
            "apps": [{"name": "A", "folderName": "A", "repository": "o/a"}, {"name": 5}]}"#,
    )
    .expect("a valid list");
    assert_eq!(list.name.as_deref(), Some("Nintendo"));
    assert_eq!(list.description.as_deref(), Some("Ports"));
    assert_eq!(list.version, "1.0.19");
    assert_eq!(list.icon_url.as_deref(), Some("https://x.test/i.png"));
    assert_eq!(list.preferred_tag_filters, ["recomp", "n64"], "tags are normalised");
    assert_eq!(list.hidden_tag_filters, ["decompilation"]);
    assert!(list.featured_tags.is_empty());
    assert_eq!(list.apps.len(), 1);
    assert_eq!(list.skipped.len(), 1, "the unreadable entry is reported, not fatal");
    assert_eq!(list.skipped[0].index, 1);
}

#[test]
fn a_missing_or_numeric_version_falls_back_to_a_hash_of_the_content() {
    for text in [
        r#"{"apps": [{"name": "A", "folderName": "A", "repository": "o/a"}]}"#,
        r#"{"version": 3, "apps": [{"name": "A", "folderName": "A", "repository": "o/a"}]}"#,
    ] {
        let list = parse_list(text).expect("valid");
        assert_eq!(list.version.len(), 64);
        assert!(list.version.chars().all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c)), "{}", list.version);
    }
}

#[test]
fn the_hash_ignores_order_and_mods_but_not_content() {
    let (a, b) = (app("o/a", "A"), app("o/b", "B"));
    assert_eq!(content_hash(&[a.clone(), b.clone()]), content_hash(&[b.clone(), a.clone()]));
    let moddable = AppEntry { mods: crate::mods::ModsConfig { path: "mods".into(), ..Default::default() }, ..a.clone() };
    assert_eq!(content_hash(std::slice::from_ref(&a)), content_hash(&[moddable]));
    let renamed = AppEntry { name: "Other".into(), ..a.clone() };
    assert_ne!(content_hash(&[a]), content_hash(&[renamed]));
}

#[test]
fn an_icon_must_be_an_absolute_web_address_and_otherwise_is_cleared() {
    for good in ["https://a.test/i.png", " http://a.test/i.png "] {
        assert!(normalize_icon_url(Some(good)).is_some(), "{good}");
    }
    for bad in ["", "icon.png", "file:///etc/passwd", "ftp://a.test/i.png", "data:image/png;base64,AAAA", "javascript:alert(1)"] {
        assert_eq!(normalize_icon_url(Some(bad)), None, "{bad}");
    }
    assert_eq!(normalize_icon_url(None), None);
}

#[test]
fn legacy_shapes_still_read() {
    let sections = parse_list(
        r#"{"standard": [{"name": "A", "folderName": "A", "repository": "o/a"}], "custom": [{"name": "B", "folderName": "B"}]}"#,
    )
    .expect("legacy");
    assert_eq!(sections.apps.len(), 2);
    let bare = parse_list(r#"[{"name": "A", "folderName": "A", "repository": "o/a"}]"#).expect("a bare array");
    assert_eq!((bare.apps.len(), bare.name), (1, None));
}

#[test]
fn bad_documents_are_errors_and_a_bom_is_fine() {
    for bad in ["", "{", "42", "\"x\""] {
        assert!(parse_list(bad).is_err(), "{bad:?}");
    }
    assert!(parse_list("\u{feff}{\"apps\": []}").is_ok());
    // A list with no apps is a valid empty list; the library, which must never lose entries, is stricter.
    assert!(parse_list("{}").expect("empty").apps.is_empty());
}

#[test]
fn a_list_with_no_name_is_named_from_where_it_came_from() {
    for (location, name) in [
        ("https://raw.example.test/x/community-app-catalog/Nintendo.json", "Nintendo"),
        ("https://e.test/my-cool-list.json?raw=1", "my cool list"),
        ("/home/me/lists/other-platforms.json", "other platforms"),
        ("C:\\lists\\mine.json", "mine"),
        ("", ""),
    ] {
        assert_eq!(name_from_location(location), name, "{location}");
    }
}

#[test]
fn long_versions_are_shortened_for_display_and_short_ones_are_not() {
    assert_eq!(version_for_display("1.0.19"), "1.0.19");
    assert_eq!(version_for_display("  "), "unknown");
    let hash = "0123456789ABCDEFFEDCBA9876543210";
    assert_eq!(version_for_display(hash), "01234567…76543210");
    assert_eq!(version_for_display("exactly16chars!!"), "exactly16chars!!");
}
