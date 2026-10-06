use serde_json::json;

use super::*;

#[test]
fn addresses_are_built_with_their_parts_encoded() {
    assert_eq!(
        listing_url("https://thunderstore.io/", "zelda-64-recompiled", 2, Sort::MostDownloaded, Some("abc")),
        "https://thunderstore.io/api/cyberstorm/listing/zelda-64-recompiled/?page=2&ordering=most-downloaded&nsfw=false&deprecated=false&section=abc"
    );
    assert!(!listing_url("https://t", "c", 0, Sort::Newest, Some(" ")).contains("section"));
    assert!(listing_url("https://t", "c", 0, Sort::Newest, None).contains("page=1&ordering=newest"));
    assert_eq!(package_url("https://t", "Owner", "My Mod"), "https://t/api/experimental/package/Owner/My%20Mod/");
    assert_eq!(filters_url("https://t", "starship"), "https://t/api/cyberstorm/community/starship/filters/");
}

#[test]
fn the_mods_section_is_found_by_slug_or_name() {
    let body =
        json!({"sections": [{"uuid": "u1", "name": "Modpacks", "slug": "modpacks"}, {"uuid": "u2", "name": "Mods", "slug": "mods"}]});
    assert_eq!(parse_mods_section(body.to_string().as_bytes()).as_deref(), Some("u2"));
    let by_name = json!({"sections": [{"uuid": "u3", "name": "MODS"}]});
    assert_eq!(parse_mods_section(by_name.to_string().as_bytes()).as_deref(), Some("u3"));
    assert_eq!(parse_mods_section(b"{}"), None);
    assert_eq!(parse_mods_section(b"not json"), None);
}

#[test]
fn a_listing_page_becomes_packages() {
    let body = json!({
        "count": 41,
        "next": "https://thunderstore.io/api/cyberstorm/listing/zelda-64-recompiled/?ordering=most-downloaded&page=3",
        "results": [
            {"name": "Better_Camera", "namespace": "Cam", "description": "Free camera", "download_count": 12904, "icon_url": "https://gcdn.thunderstore.io/live/repository/icons/Cam-Better_Camera-1.2.0.png", "is_deprecated": false, "is_nsfw": false, "rating_count": 30, "size": 52000},
            {"name": "", "namespace": "Nobody"},
            {"name": "Old", "namespace": "X", "is_deprecated": true}
        ]
    });
    let page = parse_listing(body.to_string().as_bytes(), "zelda-64-recompiled").expect("parses");
    assert_eq!((page.next, page.total, page.packages.len()), (Some(3), Some(41), 2));
    let first = &page.packages[0];
    assert_eq!((first.id.as_str(), first.owner.as_str(), first.name.as_str()), ("Cam-Better_Camera", "Cam", "Better_Camera"));
    assert_eq!((first.version.as_str(), first.downloads, first.rating, first.size), ("1.2.0", 12904, 30, Some(52000)));
    assert_eq!(first.page_url.as_deref(), Some("https://thunderstore.io/c/zelda-64-recompiled/p/Cam/Better_Camera/"));
    assert!(page.packages[1].deprecated && page.packages[1].version.is_empty());
    assert!(parse_listing(b"[]", "c").is_err());
    assert!(parse_listing(json!({"results": [], "next": null}).to_string().as_bytes(), "c").is_ok_and(|p| p.next.is_none()));
}

#[test]
fn the_version_hint_comes_from_the_icon_name() {
    assert_eq!(version_from_icon("https://x/icons/A-B_C-2.0.1.png", "A", "B_C").as_deref(), Some("2.0.1"));
    assert_eq!(version_from_icon("https://x/icons/a-b_c-2.0.1.png", "A", "B_C").as_deref(), Some("2.0.1"));
    assert_eq!(version_from_icon("https://x/icons/Other-2.0.1.png", "A", "B"), None);
    assert_eq!(version_from_icon("https://x/icons/A-B-.png", "A", "B"), None);
}

#[test]
fn a_package_answer_gives_the_download_and_its_dependencies() {
    let body = json!({
        "namespace": "Cam", "name": "Better_Camera", "full_name": "Cam-Better_Camera", "is_deprecated": false,
        "latest": {"version_number": "1.2.0", "download_url": "https://thunderstore.io/package/download/Cam/Better_Camera/1.2.0/", "dependencies": ["Lib-Core-2.1.0", ""], "description": "Free camera", "icon": "https://x/i.png", "downloads": 5}
    });
    let (package, download) = parse_package(body.to_string().as_bytes(), "zelda-64-recompiled").expect("parses");
    assert_eq!(package.id, "Cam-Better_Camera");
    assert_eq!((download.version.as_str(), download.dependencies.as_slice()), ("1.2.0", ["Lib-Core-2.1.0".to_string()].as_slice()));
    assert!(download.url.ends_with("/1.2.0/"));
    assert_eq!(download.file_name.as_deref(), Some("Cam-Better_Camera-1.2.0.zip"));
    assert!(parse_package(json!({"namespace": "a", "name": "b"}).to_string().as_bytes(), "c").is_err(), "no version");
    assert!(
        parse_package(json!({"namespace": "a", "name": "b", "latest": {"version_number": "1.0.0"}}).to_string().as_bytes(), "c").is_err()
    );
}

#[test]
fn dependency_strings_are_split_at_the_version() {
    assert_eq!(parse_dependency("Lib-Core-2.1.0"), Some(("Lib-Core".into(), Some("2.1.0".into()))));
    assert_eq!(parse_dependency("Lib-Core-Extra-1.0"), Some(("Lib-Core-Extra".into(), Some("1.0".into()))));
    assert_eq!(parse_dependency("Lib-Core"), Some(("Lib-Core".into(), None)));
    assert_eq!(parse_dependency("Lib-Core-v2"), Some(("Lib-Core-v2".into(), None)));
    assert_eq!(parse_dependency("Lonely"), None);
    assert_eq!(split_full_name("Lib-Core-Extra"), Some(("Lib", "Core-Extra")));
    assert_eq!(split_full_name("-x"), None);
}
