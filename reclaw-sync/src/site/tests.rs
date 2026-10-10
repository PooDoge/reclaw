use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use reclaw_net::{
    Net,
    testing::{Reply, TestServer, local_config},
};

use super::*;

fn net(dir: Option<&tempfile::TempDir>) -> Net {
    let mut config = local_config();
    config.cache_dir = dir.map(|d| d.path().to_path_buf());
    config.attempts = 1;
    Net::new(config).expect("net")
}

/// Two pages of apps and one of the feed, as the API pages them.
fn catalog(req: &reclaw_net::testing::Req) -> Reply {
    match req.path.as_str() {
        "/api/v1/release-status?limit=100" => Reply::ok(
            r#"{"items":[{"id":"k1","slug":"sm64","provider":"github","repository":"o/sm64"},{"id":"k2","slug":"zelda","provider":"github","repository":"o/zelda"}],"isDone":true}"#,
        ),
        "/api/v1/apps?limit=100" => {
            Reply::ok(r#"{"items":[{"slug":"sm64","name":"Super Mario 64","recommended":4}],"nextCursor":"p 2","isDone":false}"#)
        }
        "/api/v1/apps?limit=100&cursor=p%202" => Reply::ok(r#"{"items":[{"slug":"zelda","name":"Zelda","reportBroken":1}],"isDone":true}"#),
        "/api/v1/apps/sm64" => Reply::ok(r#"{"entry":{"slug":"sm64"},"project":{"author":"Ghostship","provider":"github"}}"#),
        "/api/v1/apps/sm64/reviews?limit=10" => {
            Reply::ok(r#"{"items":[{"author":"Sam","result":"runs","body":"Perfect.","createdAt":1}],"isDone":true}"#)
        }
        "/api/v1/apps/sm64/release-history?limit=100" => Reply::ok(r#"{"items":[{"version":"v2","state":"verified"}],"isDone":true}"#),
        _ => Reply::new(404, r#"{"error":{"message":"Not found"}}"#),
    }
}

#[test]
fn the_links_read_every_page_of_the_listing_and_the_feed() {
    let server = TestServer::start(|req, _| catalog(req));
    let links = SiteClient::with_base(net(None), server.url("/api/v1/")).links().expect("read");
    assert_eq!(links.status.len(), 2);
    assert_eq!(links.app("SM64").map(|a| a.recommended), Some(4));
    assert_eq!(links.app("zelda").map(|a| a.report_broken), Some(1), "the second page, reached by its cursor");
}

#[test]
fn an_apps_page_reviews_and_releases_are_read_and_a_missing_app_is_none() {
    let server = TestServer::start(|req, _| catalog(req));
    let client = SiteClient::with_base(net(None), server.url("/api/v1"));
    let detail = client.detail("sm64").expect("read").expect("listed");
    assert_eq!(detail.project.author.as_deref(), Some("Ghostship"));
    assert_eq!(client.reviews("sm64").expect("read")[0].body, "Perfect.");
    assert_eq!(client.release_history("sm64").expect("read")[0].version, "v2");
    assert_eq!(client.detail("gone").expect("a 404 is an answer"), None);
    assert_eq!(client.release_history("gone").expect("a site without the list"), Vec::new());
    assert!(matches!(client.reviews("gone"), Err(SiteError::Net(NetError::Status { status: 404, .. }))));
}

#[test]
fn a_cursor_that_never_ends_stops_after_the_page_limit() {
    let server = TestServer::start(|req, n| {
        if req.path.starts_with("/release-status") {
            Reply::ok(format!(r#"{{"items":[],"nextCursor":"c{n}","isDone":false}}"#))
        } else {
            Reply::ok(r#"{"items":[],"isDone":true}"#)
        }
    });
    assert_eq!(SiteClient::with_base(net(None), server.url("")).links(), Err(SiteError::Endless));
}

#[test]
fn the_listing_is_used_offline_from_what_was_saved() {
    let dir = tempfile::tempdir().expect("dir");
    let up = Arc::new(AtomicUsize::new(1));
    let answers = up.clone();
    let server = TestServer::start(move |req, _| if answers.load(Ordering::SeqCst) == 1 { catalog(req) } else { Reply::new(503, "down") });
    let client = SiteClient::with_base(net(Some(&dir)), server.url("/api/v1"));
    assert_eq!(client.links().expect("online").apps.len(), 2);
    up.store(0, Ordering::SeqCst);
    let again = SiteClient::with_base(net(Some(&dir)), server.url("/api/v1")).links().expect("the saved copy");
    assert_eq!(again.apps.len(), 2);
}

#[test]
fn an_unreachable_api_falls_back_to_the_deployment_host_and_stays_there() {
    let deployment = TestServer::start(|req, _| catalog(req));
    // Nothing listens on port 1: a connection error, the way a failed handshake leaves no answer either.
    let client = SiteClient::with_fallback(net(None), "http://127.0.0.1:1/api/v1", &deployment.url("/api/v1"));
    assert!(client.detail("sm64").expect("from the fallback").is_some());
    assert!(client.reviews("sm64").is_ok());
    assert_eq!(deployment.count(), 2, "both asked the deployment's host");
}

#[test]
fn an_api_that_answers_with_an_error_is_not_routed_around() {
    let api = TestServer::start(|_, _| Reply::new(500, "broken"));
    let deployment = TestServer::start(|req, _| catalog(req));
    let client = SiteClient::with_fallback(net(None), &api.url("/api/v1"), &deployment.url("/api/v1"));
    assert!(matches!(client.detail("sm64"), Err(SiteError::Net(NetError::Status { status: 500, .. }))));
    assert_eq!(deployment.count(), 0, "a status is an answer: the site is up and said no");
}
