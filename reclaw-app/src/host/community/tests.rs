use std::sync::Arc;

use reclaw_net::{
    Net,
    testing::{Reply, TestServer, local_config},
};
use reclaw_sync::{CatalogSync, SiteClient};
use reclaw_ui::{community::PageState, effect::Effect, store::AppAction};

use crate::host::{Host, HostConfig, tests::Collector};

/// The community catalog (one list of two apps) and quiverlauncher.com's API, on one server.
fn server() -> TestServer {
    TestServer::start(server_reply)
}

fn server_reply(req: &reclaw_net::testing::Req, _: usize) -> Reply {
    {
        let host = req.header("host").unwrap_or("127.0.0.1").to_string();
        match req.path.as_str() {
            "/index.json" => Reply::ok(format!(r#"{{"version": 2, "lists": [{{"id": "n", "remoteLocation": "http://{host}/n.json"}}]}}"#)),
            "/n.json" => Reply::ok(
                r#"{"name": "Nintendo", "version": "1", "apps": [
                  {"name": "One", "repository": "o/one", "folderName": "One", "tags": ["n64"]},
                  {"name": "Two", "repository": "o/two", "folderName": "Two", "tags": ["n64"]}]}"#,
            ),
            "/api/v1/release-status?limit=100" => {
                Reply::ok(r#"{"items":[{"id":"k1","slug":"one","provider":"github","repository":"O/One"}],"isDone":true}"#)
            }
            "/api/v1/apps?limit=100" => Reply::ok(
                r#"{"items":[{"id":"k1","slug":"one","name":"One Deluxe","recommended":3,"reportIssues":1,"launcher":{"folderName":"One"}}],"isDone":true}"#,
            ),
            "/api/v1/apps/one" => Reply::ok(r#"{"entry":{"slug":"one"},"project":{"author":"Ghostship","provider":"github"}}"#),
            "/api/v1/apps/one/reviews?limit=10" => {
                Reply::ok(r#"{"items":[{"author":"Sam","result":"runs","body":"Great."}],"isDone":true}"#)
            }
            _ => Reply::new(404, "no"),
        }
    }
}

fn open(server: &TestServer) -> (Host, Arc<Collector>, CatalogSync, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = local_config();
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    let sink = Arc::new(Collector::default());
    let (host, _) = Host::open(
        HostConfig {
            index_url: Some(server.url("/index.json")),
            site_api: Some(server.url("/api/v1")),
            ..HostConfig::new(Some(net.clone()), dir.path().join("apps.json"))
        },
        sink.clone(),
    );
    let sync = CatalogSync::new(net.clone())
        .with_index_url(server.url("/index.json"))
        .with_site(SiteClient::with_base(net, server.url("/api/v1")));
    (host, sink, sync, dir)
}

fn linked(sink: &Collector) -> Option<std::collections::BTreeMap<u32, reclaw_catalog::site::SiteApp>> {
    sink.all().into_iter().rev().find_map(|a| if let AppAction::SetCommunity(apps) = a { Some(apps) } else { None })
}

#[test]
fn a_refresh_links_the_games_the_site_lists_and_a_page_is_read_when_it_opens() {
    let server = server();
    let (host, sink, sync, _dir) = open(&server);
    host.run_refresh(&sync);
    sink.wait_for("the games linked", |all| all.iter().any(|a| matches!(a, AppAction::SetCommunity(_))));
    let apps = linked(&sink).expect("sent");
    assert_eq!(apps.len(), 1, "Two is not on the site");
    assert!(sink.notices().is_empty(), "{:?}", sink.notices());
    let (&one, app) = apps.iter().next().expect("One");
    assert_eq!((app.recommended, app.report_issues), (3, 1));

    host.handle(&Effect::LoadCommunity(one));
    sink.wait_for("the page", |all| all.iter().any(|a| matches!(a, AppAction::CommunityPage { page: PageState::Loaded(_), .. })));
    let page = sink
        .all()
        .into_iter()
        .find_map(|a| match a {
            AppAction::CommunityPage { id, page: PageState::Loaded(page) } if id == one => Some(page),
            _ => None,
        })
        .expect("loaded");
    let detail = page.detail.expect("read").expect("listed");
    assert_eq!(detail.project.author.as_deref(), Some("Ghostship"));
    assert_eq!(page.reviews.expect("read")[0].body, "Great.");
    assert_eq!(page.releases, Ok(Vec::new()), "a site without release history has none");
    assert!(sink.all().iter().any(|a| matches!(a, AppAction::CommunityPage { page: PageState::Loading, .. })), "Loading came first");
}

#[test]
fn a_game_with_no_entry_asks_nothing() {
    let server = server();
    let (host, sink, sync, _dir) = open(&server);
    host.handle(&Effect::LoadCommunity(42));
    assert!(sink.all().is_empty(), "nothing is linked yet");

    drop((sync, host));
}

#[test]
fn a_site_that_is_down_leaves_the_lists_and_says_newer_apps_are_missing() {
    let server = TestServer::start(|req, n| if req.path.starts_with("/api/") { Reply::new(500, "down") } else { server_reply(req, n) });
    let (host, sink, sync, _dir) = open(&server);
    host.run_refresh(&sync);
    assert!(linked(&sink).is_none(), "nothing to link with");
    let notice = sink.notices().into_iter().next().expect("told");
    assert!(notice.details.iter().any(|d| d.contains("quiverlauncher.com")), "{notice:?}");
}

#[test]
fn a_library_app_from_the_site_follows_its_name_and_remembers_what_it_set() {
    let server = server();
    let (host, sink, sync, dir) = open(&server);
    host.run_refresh(&sync);
    let one = sink
        .all()
        .into_iter()
        .rev()
        .find_map(|a| if let AppAction::SetCommunity(apps) = a { apps.keys().next().copied() } else { None })
        .expect("linked");
    host.handle(&Effect::AddToLibrary(one));
    host.run_refresh(&sync);
    let saved = std::fs::read_to_string(dir.path().join("apps.json")).expect("saved");
    assert!(saved.contains(r#""catalogEntryId": "k1""#), "{saved}");
    assert!(saved.contains(r#""name": "One Deluxe""#), "the site's name: {saved}");
}
