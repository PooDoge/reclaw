use serde_json::json;

use super::*;

#[test]
fn the_index_address_filters_by_game() {
    assert_eq!(
        index_url("https://gamebanana.com/", "24290", 0, Sort::MostDownloaded),
        "https://gamebanana.com/apiv13/Mod/Index?_nPerpage=50&_aFilters%5BGeneric_Game%5D=24290&_nPage=1&_sSort=Generic_MostDownloaded"
    );
    assert!(detail_url("https://g", "123").starts_with("https://g/apiv11/Mod/123?_csvProperties="));
}

#[test]
fn an_index_page_keeps_free_mods_only() {
    let body = json!({
        "_aMetadata": {"_nRecordCount": "120", "_bIsComplete": false},
        "_aRecords": [
            {"_idRow": 501, "_sModelName": "Mod", "_sName": "HD Textures", "_sProfileUrl": "https://gamebanana.com/mods/501", "_sVersion": "1.1",
             "_nLikeCount": 7, "_nDownloadCount": "900", "_aSubmitter": {"_sName": "Ana"}, "_tsDateAdded": 1700000000, "_tsDateUpdated": 1709648551,
             "_aPreviewMedia": {"_aImages": [{"_sBaseUrl": "https://images.gamebanana.com/img/ss/mods", "_sFile220": "220-90_x.jpg", "_sFile530": "530-90_x.jpg"}]}},
            {"_idRow": "502", "_sName": "Rated", "_bHasContentRatings": true, "_tsDateModified": "1709000000",
             "_aPreviewContent": {"screenshot": {"_sBaseUrl": "https://i/", "_sFile530": "a.jpg", "_sFile530Sfw": "a-sfw.jpg"}}},
            {"_idRow": 503, "_sModelName": "Tool", "_sName": "Editor"},
            {"_idRow": 504, "_sName": "Paid", "_sPayType": "paid"},
            {"_idRow": 0, "_sName": "Nothing"}
        ]
    });
    let page = parse_index(body.to_string().as_bytes(), "24290", 1).expect("parses");
    assert_eq!((page.next, page.total, page.packages.len()), (Some(2), Some(120), 2));
    let first = &page.packages[0];
    assert_eq!((first.id.as_str(), first.full_name.as_str(), first.version.as_str()), ("501", "Ana-HD Textures", "1.1"));
    assert_eq!((first.downloads, first.rating), (900, 7));
    assert_eq!((first.created, first.updated), (Some(1_700_000_000), Some(1_709_648_551)));
    assert_eq!((page.packages[1].created, page.packages[1].updated), (None, Some(1_709_000_000)), "modified stands in for updated");
    assert_eq!(first.icon_url.as_deref(), Some("https://images.gamebanana.com/img/ss/mods/530-90_x.jpg"));
    assert_eq!(page.packages[1].icon_url.as_deref(), Some("https://i/a-sfw.jpg"), "the safe crop of a rated mod");
    assert_eq!(page.packages[1].page_url.as_deref(), Some("https://gamebanana.com/mods/502"));

    let last = json!({"_aMetadata": {"_bIsComplete": true}, "_aRecords": [{"_idRow": 1, "_sName": "x"}]});
    assert_eq!(parse_index(last.to_string().as_bytes(), "1", 3).expect("parses").next, None);
    assert_eq!(parse_index(json!({"_aRecords": []}).to_string().as_bytes(), "1", 1).expect("parses").next, None);
    assert!(parse_index(b"{}", "1", 1).is_err());
}

#[test]
fn the_download_is_the_first_archive_still_offered() {
    let body = json!({
        "_sVersion": "2.0",
        "_aFiles": [
            {"_idRow": 1, "_sFile": "old.zip", "_sDownloadUrl": "https://gamebanana.com/dl/1", "_bIsArchived": true},
            {"_idRow": 2, "_sFile": "readme.txt", "_sDownloadUrl": "https://gamebanana.com/dl/2"},
            {"_idRow": 3, "_sFile": "mod.7z", "_sDownloadUrl": "https://gamebanana.com/dl/3", "_nFilesize": 1234, "_sVersion": ""},
            {"_idRow": 4, "_sFile": "no-url.zip"}
        ]
    });
    let detail = parse_detail(body.to_string().as_bytes()).expect("parses");
    assert_eq!(detail.files.len(), 2);
    let download = choose_download(&detail).expect("a file");
    assert_eq!((download.url.as_str(), download.version.as_str(), download.size), ("https://gamebanana.com/dl/3", "2.0", Some(1234)));
    assert_eq!((download.file_name.as_deref(), download.file_id.as_deref()), (Some("mod.7z"), Some("3")));

    let only_text = Detail {
        version: String::new(),
        files: vec![File { id: "9".into(), name: "x.nrm".into(), url: "u".into(), size: None, version: String::new() }],
    };
    assert_eq!(choose_download(&only_text).map(|d| d.version), Some("0".to_string()), "a file that is not an archive is still offered");
    assert_eq!(choose_download(&Detail::default()), None);
    assert!(parse_detail(b"[]").is_err());
}
