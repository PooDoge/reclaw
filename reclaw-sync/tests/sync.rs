//! The whole sync against a server on this machine that plays the catalog's host: the index, the lists, the platform metadata.
use std::{fs, path::PathBuf, time::Duration};

use reclaw_catalog::RepoSource;
use reclaw_net::{
    Net, NetConfig,
    testing::{Reply, TestServer, local_config},
};
use reclaw_sync::{CatalogSync, Origin, SiteClient, SyncError};

const PLATFORM: &str = r#"{"formatRevision": 1, "generatedAt": "2026-10-04T12:29:25+00:00", "entries": [
  {"provider": "github", "repository": "o/one", "preferredRelease": null, "releaseTag": "v1.2.0", "assetNames": ["one-windows.zip", "one-linux.tar.gz"], "validatedAt": "2026-10-04T12:28:25+00:00", "selectionRevision": 1}
]}"#;

fn list(name: &str, apps: &[(&str, &str)]) -> String {
    let apps: Vec<String> =
        apps.iter().map(|(n, r)| format!(r#"{{"name": "{n}", "repository": "{r}", "folderName": "{n}", "tags": ["n64"]}}"#)).collect();
    format!(r#"{{"name": "{name}", "version": "1.0.0", "apps": [{}]}}"#, apps.join(","))
}

/// A server with an index of two lists and the platform metadata; `tweak` can change any answer.
fn catalog_server(tweak: impl Fn(&str, Reply) -> Reply + Send + Sync + 'static) -> TestServer {
    TestServer::start(move |req, _| {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        let reply = match req.path.as_str() {
            "/index.json" => Reply::ok(format!(
                r#"{{"version": 2, "lists": [{{"id": "n", "remoteLocation": "http://{host}/n.json"}}, {{"id": "o", "remoteLocation": "http://{host}/o.json"}}], "platformMetadataUrl": "http://{host}/platform.json"}}"#
            )),
            "/n.json" => Reply::ok(list("Nintendo", &[("One", "o/one"), ("Two", "o/two")])),
            "/o.json" => Reply::ok(list("Other", &[("Three", "o/three"), ("Two", "O/TWO")])),
            "/platform.json" => Reply::ok(PLATFORM),
            _ => Reply::new(404, "no"),
        };
        tweak(&req.path, reply)
    })
}

fn sync_for(server: &TestServer, dir: Option<&tempfile::TempDir>) -> CatalogSync {
    let mut config = local_config();
    config.cache_dir = dir.map(|d| d.path().to_path_buf());
    config.attempts = 1;
    CatalogSync::new(Net::new(config).expect("net")).with_index_url(server.url("/index.json"))
}

#[test]
fn a_refresh_builds_the_snapshot_from_all_three_kinds_of_file() {
    let server = catalog_server(|_, r| r);
    let snapshot = sync_for(&server, None).refresh().expect("loaded");
    assert_eq!(snapshot.lists.len(), 2);
    assert!(snapshot.problems.is_empty(), "{:?}", snapshot.problems);
    assert_eq!(snapshot.index_origin, Origin::Network);
    let apps = snapshot.apps();
    let names: Vec<_> = apps.iter().map(|a| a.entry.name.as_str()).collect();
    assert_eq!(names, ["One", "Two", "Three"], "the second list's Two is the first list's Two: same repository, same folder");
    assert_eq!(apps[1].lists, ["Nintendo", "Other"]);
    let release = apps[0].release.as_ref().expect("platform metadata covers One");
    assert_eq!((release.release_tag.as_str(), release.asset_names.len()), ("v1.2.0", 2));
    assert!(apps[2].release.is_none(), "an app the metadata does not cover is unknown, not excluded");
}

#[test]
fn a_list_that_fails_is_reported_and_the_others_still_load() {
    let server = catalog_server(|path, r| if path == "/o.json" { Reply::new(404, "gone") } else { r });
    let snapshot = sync_for(&server, None).refresh().expect("the rest loaded");
    assert_eq!(snapshot.lists.len(), 1);
    assert_eq!(snapshot.apps().len(), 2);
    assert_eq!(snapshot.problems.len(), 1);
    assert!(snapshot.problems[0].what.contains('o') && snapshot.problems[0].detail.contains("404"), "{:?}", snapshot.problems);
}

#[test]
fn a_list_that_is_not_json_and_one_with_unreadable_entries_are_told_apart() {
    let server = catalog_server(|path, r| match path {
        "/n.json" => Reply::ok("<html>not json"),
        "/o.json" => Reply::ok(
            r#"{"name": "Other", "version": "1", "apps": [{"name": "Good", "repository": "o/good", "folderName": "Good"}, {"name": 5}]}"#,
        ),
        _ => r,
    });
    let snapshot = sync_for(&server, None).refresh().expect("loaded");
    assert_eq!(snapshot.apps().len(), 1, "the good entry of the damaged list survives");
    let text: Vec<_> = snapshot.problems.iter().map(|p| p.detail.as_str()).collect();
    assert!(text.iter().any(|t| t.contains("could not be read")), "{text:?}");
    assert!(text.iter().any(|t| t.contains("1 entries that could not be read")), "{text:?}");
}

#[test]
fn platform_metadata_that_is_invalid_or_from_the_future_is_left_out_with_a_reason() {
    let invalid = catalog_server(|path, r| {
        if path == "/platform.json" {
            Reply::ok(r#"{"formatRevision": 9, "generatedAt": "2026-10-04T12:29:25Z", "entries": []}"#)
        } else {
            r
        }
    });
    let snapshot = sync_for(&invalid, None).refresh().expect("loaded");
    assert!(
        snapshot.platform.is_none() && snapshot.problems.iter().any(|p| p.what.contains("platform") && p.detail.contains("not valid")),
        "{:?}",
        snapshot.problems
    );
    assert!(snapshot.apps().iter().all(|a| a.release.is_none()), "no metadata is not the same as a wrong guess");

    let future = catalog_server(|path, r| {
        if path == "/platform.json" { Reply::ok(PLATFORM.replace("2026-10-04T12:29:25", "2999-01-01T00:00:00")) } else { r }
    });
    let snapshot = sync_for(&future, None).refresh().expect("loaded");
    assert!(snapshot.platform.is_none() && snapshot.problems.iter().any(|p| p.detail.contains("future")), "{:?}", snapshot.problems);
}

#[test]
fn no_index_is_an_error_and_a_broken_one_is_a_different_error() {
    let down = TestServer::start(|_, _| Reply::new(503, "down"));
    assert!(matches!(sync_for(&down, None).refresh(), Err(SyncError::NoIndex(_))));
    let junk = TestServer::start(|_, _| Reply::ok("{\"version\": 2, \"lists\": []}"));
    let err = sync_for(&junk, None).refresh().expect_err("empty index");
    assert!(matches!(err, SyncError::BadIndex(_)), "{err:?}");
}

#[test]
fn what_was_loaded_can_be_shown_again_with_no_network_at_all() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = catalog_server(|_, r| r);
    let sync = sync_for(&server, Some(&dir));
    assert!(sync.saved().is_none(), "nothing yet");
    let online = sync.refresh().expect("online");
    let before = server.count();
    let offline = sync.saved().expect("saved copies");
    assert_eq!(server.count(), before, "no request was made");
    assert_eq!(offline.apps(), online.apps());
    assert!(offline.platform.is_some());
}

#[test]
fn when_the_host_goes_away_the_saved_copies_stand_in_and_the_snapshot_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sync = {
        let server = catalog_server(|_, r| r);
        let sync = sync_for(&server, Some(&dir)).with_ttl(Duration::ZERO);
        sync.refresh().expect("online");
        sync
    };
    std::thread::sleep(Duration::from_millis(100));
    let snapshot = sync.refresh().expect("served from what was saved");
    assert!(snapshot.has_stale_parts());
    assert_eq!(snapshot.apps().len(), 3);
    assert!(snapshot.problems.iter().any(|p| p.detail.contains("saved copy")), "{:?}", snapshot.problems);
}

#[test]
fn a_second_refresh_inside_the_lifetime_asks_nothing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = catalog_server(|_, r| r);
    let sync = sync_for(&server, Some(&dir)).with_ttl(Duration::from_secs(3600));
    sync.refresh().expect("first");
    let after_first = server.count();
    let second = sync.refresh().expect("second");
    assert_eq!(server.count(), after_first);
    assert_eq!(second.index_origin, Origin::Saved);
}

#[test]
fn the_real_catalog_served_from_this_machine_loads_completely() {
    let Some(dir) = std::env::var_os("QUIVER_CATALOG_DIR").map(PathBuf::from) else {
        eprintln!("QUIVER_CATALOG_DIR is not set: skipping the real-catalog check");
        return;
    };
    let server = TestServer::start({
        let dir = dir.clone();
        move |req, _| {
            let host = req.header("host").unwrap_or("127.0.0.1").to_string();
            let file = match req.path.as_str() {
                "/index.json" => {
                    // The real index points at the real web; point it at this server instead.
                    let text = fs::read_to_string(dir.join("index.json")).unwrap_or_default();
                    return Reply::ok(text.replace(
                        "https://raw.githubusercontent.com/tgeorgiadis/quiver-community-app-catalog/main/",
                        &format!("http://{host}/"),
                    ));
                }
                path => path.trim_start_matches('/').to_string(),
            };
            fs::read(dir.join(&file)).map_or_else(|_| Reply::new(404, "no"), Reply::ok)
        }
    });
    let snapshot = sync_for(&server, None).refresh().expect("the real catalog loads");
    assert!(snapshot.problems.is_empty(), "{:#?}", snapshot.problems);
    let apps = snapshot.apps();
    let hosted = apps.iter().filter(|a| !a.entry.is_manual()).count();
    let with_release = apps.iter().filter(|a| a.release.is_some()).count();
    eprintln!("{} lists, {} apps, {with_release} with release metadata", snapshot.lists.len(), apps.len());
    assert_eq!(snapshot.lists.len(), 4);
    assert!(apps.len() >= 200, "{}", apps.len());
    assert_eq!(with_release, hosted, "every hosted app is covered by the platform metadata");
}

/// Talks to the real internet, so it is ignored by default:
///
///   cargo test -p reclaw-sync --test sync -- --ignored --nocapture
#[test]
#[ignore = "needs the internet"]
fn the_live_catalog_loads_from_the_web_and_a_second_start_is_instant() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (mut config, problems) = NetConfig::from_env(|k| std::env::var(k).ok());
    assert!(problems.is_empty(), "{problems:?}");
    config.cache_dir = Some(dir.path().to_path_buf());
    let sync = CatalogSync::new(Net::new(config).expect("net"));

    let started = std::time::Instant::now();
    let first = sync.refresh().expect("the live catalog loads");
    let cold = started.elapsed();
    let apps = first.apps();
    eprintln!(
        "cold: {} lists, {} apps, {} with releases, {cold:?}; problems: {:?}",
        first.lists.len(),
        apps.len(),
        apps.iter().filter(|a| a.release.is_some()).count(),
        first.problems
    );
    assert!(apps.len() > 200 && first.lists.len() >= 4);
    assert!(first.problems.is_empty(), "{:?}", first.problems);

    let started = std::time::Instant::now();
    let saved = sync.saved().expect("saved on disk");
    let warm = started.elapsed();
    eprintln!("from disk: {} apps in {warm:?}", saved.apps().len());
    assert_eq!(saved.apps(), apps);
    assert!(warm < Duration::from_millis(500), "{warm:?}");
}

/// The site's API on the same test server: one new app the lists never had, and One, which they do.
fn site_answer(path: &str) -> Option<Reply> {
    match path {
        "/api/v1/release-status?limit=100" => Some(Reply::ok(
            r#"{"items":[{"id":"k1","slug":"one","provider":"github","repository":"o/one"},{"id":"k9","slug":"fresh","provider":"gitlab","repository":"g/fresh"}],"isDone":true}"#,
        )),
        "/api/v1/apps?limit=100" => Some(Reply::ok(
            r#"{"items":[{"id":"k1","slug":"one","name":"One","addedAt":1,"launcher":{"folderName":"One"}},
                         {"id":"k9","slug":"fresh","name":"Fresh Port","addedAt":2,"recommended":3,"launcher":{"folderName":"Fresh"}}],"isDone":true}"#,
        )),
        _ => None,
    }
}

fn sync_with_site(server: &TestServer) -> CatalogSync {
    let mut config = local_config();
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    CatalogSync::new(net.clone()).with_index_url(server.url("/index.json")).with_site(SiteClient::with_base(net, server.url("/api/v1")))
}

#[test]
fn the_sites_new_apps_join_the_catalog_ahead_of_the_frozen_lists() {
    let server = catalog_server(|path, r| site_answer(path).unwrap_or(r));
    let snapshot = sync_with_site(&server).refresh().expect("loaded");
    assert!(snapshot.problems.is_empty(), "{:?}", snapshot.problems);
    let apps = snapshot.apps();
    let names: Vec<_> = apps.iter().map(|a| a.entry.name.as_str()).collect();
    assert_eq!(names, ["Fresh Port", "One", "Two", "Three"], "the site's newest first, then what only the lists have");
    assert_eq!((apps[0].entry.repository.as_str(), apps[0].entry.source), ("g/fresh", RepoSource::Gitlab));
    assert_eq!(apps[1].lists, ["quiverlauncher.com", "Nintendo"]);
    assert!(apps[1].release.is_some(), "the platform metadata still covers One");
}

#[test]
fn a_site_that_cannot_be_read_leaves_the_lists_and_says_so() {
    let server = catalog_server(|path, r| if path.starts_with("/api/") { Reply::new(500, "down") } else { r });
    let snapshot = sync_with_site(&server).refresh().expect("the lists loaded");
    assert_eq!(snapshot.apps().len(), 3);
    assert!(snapshot.site.is_none());
    assert!(snapshot.problems[0].what.contains("quiverlauncher.com"), "{:?}", snapshot.problems);
}

#[test]
fn lists_that_cannot_be_read_leave_the_site_as_the_catalog() {
    let server = catalog_server(|path, r| site_answer(path).unwrap_or(if path == "/index.json" { Reply::new(404, "gone") } else { r }));
    let snapshot = sync_with_site(&server).refresh().expect("the site is a catalog");
    let names: Vec<_> = snapshot.apps().into_iter().map(|a| a.entry.name).collect();
    assert_eq!(names, ["Fresh Port", "One"]);
    assert!(snapshot.problems.iter().any(|p| p.what == "The community lists"), "{:?}", snapshot.problems);
}
