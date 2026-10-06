use reclaw_net::testing::{Reply, TestServer, local_net};
use serde_json::json;

use super::*;

fn release(tag: &str, prerelease: bool, files: &[&str]) -> serde_json::Value {
    json!({
        "tag_name": tag, "prerelease": prerelease, "draft": false,
        "assets": files.iter().map(|f| json!({"name": f, "browser_download_url": format!("https://example.test/{tag}/{f}"), "size": 10})).collect::<Vec<_>>()
    })
}

fn source(server: &TestServer) -> ReleaseSource {
    ReleaseSource::new(local_net()).with_api(Api { github: server.url(""), gitlab: server.url("/api/v4") })
}

#[test]
fn github_is_asked_for_its_latest_release_alone() {
    let server = TestServer::start(|req, _| match req.path.as_str() {
        "/repos/o/r/releases/latest" => Reply::ok(release("v2", false, &["g-linux.tar.gz"]).to_string()),
        other => Reply::new(404, format!("unexpected {other}")),
    });
    let found = source(&server).fetch(Host::GitHub, "o/r", false, false).expect("releases");
    assert_eq!(found.list.len(), 1);
    assert_eq!(found.latest_tag.as_deref(), Some("v2"));
    assert_eq!(server.count(), 1, "one question, not two");
    let sent = &server.requests()[0];
    assert_eq!(sent.header("accept"), Some("application/vnd.github+json"));
    assert_eq!(sent.header("x-github-api-version"), Some("2022-11-28"));
}

#[test]
fn the_whole_list_is_asked_for_when_pre_releases_or_a_pin_are_wanted() {
    let server = TestServer::start(|req, _| match req.path.as_str() {
        "/repos/o/r/releases?per_page=30" => {
            Reply::ok(json!([release("v3-beta", true, &["g.zip"]), release("v2", false, &["g.zip"])]).to_string())
        }
        other => Reply::new(404, format!("unexpected {other}")),
    });
    let found = source(&server).fetch(Host::GitHub, "o/r", true, false).expect("releases");
    assert_eq!(found.list.iter().map(|r| r.tag.as_str()).collect::<Vec<_>>(), ["v3-beta", "v2"]);
    assert_eq!(found.latest_tag, None);
    assert_eq!(server.count(), 1);
}

#[test]
fn a_latest_release_without_files_falls_back_to_the_list() {
    let server = TestServer::start(|req, _| match req.path.as_str() {
        "/repos/o/r/releases/latest" => Reply::ok(release("v2", false, &["notes.json"]).to_string()),
        "/repos/o/r/releases?per_page=30" => {
            Reply::ok(json!([release("v2", false, &["notes.json"]), release("v1", false, &["g-linux.tar.gz"])]).to_string())
        }
        other => Reply::new(404, format!("unexpected {other}")),
    });
    let found = source(&server).fetch(Host::GitHub, "o/r", false, false).expect("releases");
    assert_eq!(found.list.len(), 2);
    assert_eq!(found.latest_tag.as_deref(), Some("v2"), "the host's own idea of latest is kept");
    assert_eq!(server.count(), 2);
}

#[test]
fn a_repository_with_no_latest_release_falls_back_to_the_list() {
    let server = TestServer::start(|req, _| match req.path.as_str() {
        "/repos/o/r/releases/latest" => Reply::new(404, "{}"),
        "/repos/o/r/releases?per_page=30" => Reply::ok(json!([release("v1-beta", true, &["g-linux.tar.gz"])]).to_string()),
        other => Reply::new(404, format!("unexpected {other}")),
    });
    let found = source(&server).fetch(Host::GitHub, "o/r", false, false).expect("releases");
    assert_eq!(found.list[0].tag, "v1-beta");
    assert_eq!(found.latest_tag, None);
}

#[test]
fn no_releases_at_all_is_said_plainly() {
    let server = TestServer::start(|req, _| if req.path.ends_with("latest") { Reply::new(404, "{}") } else { Reply::ok("[]") });
    assert_eq!(source(&server).fetch(Host::GitHub, "o/r", false, false), Err(InstallError::NoReleases { repo: "o/r".to_string() }));
    let missing = TestServer::start(|_, _| Reply::new(404, "{}"));
    assert_eq!(source(&missing).fetch(Host::GitHub, "o/r", true, false), Err(InstallError::NoReleases { repo: "o/r".to_string() }));
}

#[test]
fn a_rate_limit_is_passed_on_with_its_advice() {
    let server = TestServer::start(|_, _| Reply::new(429, "slow down").header("retry-after", "120"));
    let error = source(&server).fetch(Host::GitHub, "o/r", false, false).expect_err("limited");
    assert!(matches!(error, InstallError::Net(NetError::RateLimited { .. })), "{error:?}");
    assert!(error.hint().is_some_and(|h| h.contains("access token")), "{:?}", error.hint());
}

#[test]
fn an_answer_that_is_not_a_release_list_is_an_error_not_an_empty_list() {
    let server = TestServer::start(|_, _| Reply::ok("<html>Sign in</html>"));
    let error = source(&server).fetch(Host::GitHub, "o/r", true, false).expect_err("not json");
    assert!(matches!(error, InstallError::BadAnswer(_)), "{error:?}");
}

#[test]
fn gitlab_projects_travel_percent_encoded() {
    let server = TestServer::start(|req, _| {
        if req.path == "/api/v4/projects/group%2Fsub%2Fproj/releases" {
            Reply::ok(json!([{"tag_name": "v1", "assets": {"links": [{"name": "g-linux.tar.gz", "url": "https://gitlab.com/x/-/jobs/1/artifacts/download"}]}}]).to_string())
        } else {
            Reply::new(404, format!("unexpected {}", req.path))
        }
    });
    let found = source(&server).fetch(Host::GitLab, "group/sub/proj", false, false).expect("releases");
    assert_eq!(found.list.len(), 1);
    assert_eq!(found.list[0].assets.len(), 1);
}

#[test]
fn a_name_that_could_change_the_address_is_refused_before_any_request() {
    let server = TestServer::start(|_, _| Reply::ok("[]"));
    for repo in ["", "../etc/passwd", "o/r?x=1", "o//r", "/o/r", "o/r/", "o/r#frag", "o r"] {
        let error = source(&server).fetch(Host::GitHub, repo, true, false).expect_err(repo);
        assert!(matches!(error, InstallError::BadAnswer(_)), "{repo}: {error:?}");
    }
    assert_eq!(server.count(), 0);
}

#[test]
fn project_paths_are_encoded_byte_by_byte() {
    assert_eq!(encode_project("a/b-c_d.e"), "a%2Fb-c_d.e");
    assert_eq!(encode_project(" a b "), "a%20b");
}

#[test]
fn a_saved_answer_is_used_for_a_background_look_but_not_when_the_person_asked() {
    let dir = tempfile::tempdir().expect("dir");
    let server = TestServer::start(|_, _| Reply::ok(release("v1", false, &["g-linux.tar.gz"]).to_string()));
    let mut config = reclaw_net::testing::local_config();
    config.cache_dir = Some(dir.path().to_path_buf());
    let source = ReleaseSource::new(reclaw_net::Net::new(config).expect("net"))
        .with_api(Api { github: server.url(""), gitlab: server.url("/gitlab") });
    source.fetch(Host::GitHub, "o/r", false, false).expect("first");
    source.fetch(Host::GitHub, "o/r", false, false).expect("second");
    assert_eq!(server.count(), 1, "the second look used the saved answer");
    source.fetch(Host::GitHub, "o/r", false, true).expect("asked for");
    assert_eq!(server.count(), 2, "an explicit action asks again");
}
