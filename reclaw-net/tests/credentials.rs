//! Tokens in use: changed while running, refused by a service, and checked. Against a server on this machine; the host named
//! "127.0.0.1" stands in for the API host, since a token goes to the host it was given for.
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use reclaw_log::Secret;
use reclaw_net::{
    Net, NetError, Provider, Request, TokenState, Verdict,
    testing::{Reply, TestServer, local_config},
};

const HOST: &str = "127.0.0.1";

fn net_with_cache(dir: &tempfile::TempDir) -> Net {
    let mut config = local_config();
    config.cache_dir = Some(dir.path().to_path_buf());
    Net::new(config).expect("net")
}

fn cached(url: String) -> Request {
    Request::get(url).cached(Duration::from_secs(3600), false)
}

#[test]
fn a_token_set_while_running_is_used_from_the_next_request_and_answers_stay_with_their_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::start(|req, _| match req.header("authorization") {
        Some("Bearer ghp_runtime_token_0001") => Reply::ok("as the token"),
        Some(_) => Reply::new(500, "unexpected"),
        None => Reply::ok("as anyone"),
    });
    let net = net_with_cache(&dir);
    let url = server.url("/who");
    assert_eq!(net.fetch(&cached(url.clone())).expect("anon").text(), "as anyone");

    net.set_host_token(HOST, Some(Secret::new("ghp_runtime_token_0001")));
    assert_eq!(net.host_token_state(HOST), TokenState::Active);
    assert_eq!(net.fetch(&cached(url.clone())).expect("with token").text(), "as the token", "not the anonymous copy");

    net.set_host_token(HOST, None);
    assert_eq!(net.host_token_state(HOST), TokenState::None);
    assert_eq!(net.fetch(&cached(url)).expect("anon again").text(), "as anyone", "the anonymous copy is still there, and still fresh");
    assert_eq!(server.count(), 2, "two identities, one request each: the third was a cache hit");
}

#[test]
fn a_refused_token_is_dropped_and_the_request_repeated_without_it() {
    let server = TestServer::start(|req, _| {
        if req.header("authorization").is_some() { Reply::new(401, r#"{"message":"Bad credentials"}"#) } else { Reply::ok("public data") }
    });
    let mut config = local_config();
    config.tokens = vec![(HOST.into(), Secret::new("ghp_expired_token_0002"))];
    let net = Net::new(config).expect("net");
    let told: Arc<Mutex<Vec<String>>> = Arc::default();
    let sink = told.clone();
    net.on_token_rejected(move |host| sink.lock().expect("lock").push(host.to_string()));

    let got = net.fetch(&Request::get(server.url("/repos/o/r/releases"))).expect("the anonymous repeat succeeds");
    assert_eq!(got.text(), "public data");
    let seen = server.requests();
    assert_eq!(seen.len(), 2);
    assert!(seen[0].header("authorization").is_some() && seen[1].header("authorization").is_none(), "{seen:?}");
    assert_eq!(net.host_token_state(HOST), TokenState::Rejected);
    assert_eq!(*told.lock().expect("lock"), vec![HOST.to_string()], "told once, with the host");

    // Later requests do not even try the token, and the hook is not called again.
    net.fetch(&Request::get(server.url("/other"))).expect("anonymous");
    assert!(server.requests()[2].header("authorization").is_none());
    assert_eq!(told.lock().expect("lock").len(), 1);

    // A new token is trusted again.
    net.set_host_token(HOST, Some(Secret::new("ghp_fresh_token_000003")));
    assert_eq!(net.host_token_state(HOST), TokenState::Active);
}

#[test]
fn a_401_with_no_token_is_just_a_status() {
    let server = TestServer::start(|_, _| Reply::new(401, "login required"));
    let err = Net::new(local_config()).expect("net").fetch(&Request::get(server.url("/private"))).expect_err("refused");
    assert_eq!(err, NetError::Status { host: HOST.into(), status: 401 });
    assert_eq!(server.count(), 1, "nothing to retry without");
}

#[test]
fn a_wait_earned_anonymously_ends_when_a_token_is_set() {
    let server = TestServer::start(|req, _| {
        if req.header("authorization").is_some() {
            Reply::ok("with a token")
        } else {
            Reply::new(429, "slow down").header("Retry-After", "3600")
        }
    });
    let net = Net::new(local_config()).expect("net");
    let url = server.url("/x");
    assert!(matches!(net.fetch(&Request::get(url.clone())), Err(NetError::RateLimited { .. })));
    assert!(matches!(net.fetch(&Request::get(url.clone())), Err(NetError::RateLimited { .. })), "remembered: not asked again");
    assert_eq!(server.count(), 1);

    net.set_host_token(HOST, Some(Secret::new("ghp_new_allowance_0004")));
    assert_eq!(net.fetch(&Request::get(url)).expect("the new identity has its own allowance").text(), "with a token");
}

#[test]
fn many_askers_for_one_cached_answer_cost_one_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::start(|_, _| {
        std::thread::sleep(Duration::from_millis(150));
        Reply::ok("shared")
    });
    let net = net_with_cache(&dir);
    let url = server.url("/releases");
    let workers: Vec<_> = (0..8)
        .map(|_| {
            let (net, url) = (net.clone(), url.clone());
            std::thread::spawn(move || net.fetch(&cached(url)).map(|f| f.text()))
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().expect("thread").expect("fetched"), "shared");
    }
    assert_eq!(server.count(), 1, "the other seven read what the first saved");
}

#[test]
fn the_check_reads_the_quota_and_scopes_and_sends_the_token() {
    let server = TestServer::start(|req, _| {
        assert_eq!(req.header("x-github-api-version"), Some("2022-11-28"));
        let authed = req.header("authorization") == Some("Bearer ghp_check_token_000005");
        let body = if authed {
            r#"{"resources":{"core":{"limit":5000,"used":13,"remaining":4987,"reset":1790000000}}}"#
        } else {
            r#"{"resources":{"core":{"limit":60,"used":2,"remaining":58,"reset":1790000000}}}"#
        };
        Reply::ok(body).header("X-OAuth-Scopes", "").header("github-authentication-token-expiration", "2026-12-31 00:00:00 UTC")
    });
    let net = Net::new(local_config()).expect("net");
    let url = server.url("/rate_limit");

    let Verdict::Accepted(anon) = net.check_at(Provider::GitHub, &url).expect("anonymous check") else { panic!("accepted") };
    assert_eq!((anon.had_token, anon.quota.map(|q| (q.limit, q.remaining))), (false, Some((60, 58))));

    net.set_host_token(HOST, Some(Secret::new("ghp_check_token_000005")));
    let Verdict::Accepted(with) = net.check_at(Provider::GitHub, &url).expect("token check") else { panic!("accepted") };
    assert_eq!((with.had_token, with.quota.map(|q| (q.limit, q.remaining))), (true, Some((5000, 4987))));
    assert_eq!(with.scopes, Some(vec![]));
    assert_eq!(with.expires.as_deref(), Some("2026-12-31 00:00:00 UTC"));
}

#[test]
fn the_check_reports_a_refused_token_and_stops_sending_it() {
    let server = TestServer::start(|_, _| Reply::new(401, r#"{"message":"Bad credentials"}"#));
    let mut config = local_config();
    config.tokens = vec![(HOST.into(), Secret::new("ghp_revoked_token_0006"))];
    let net = Net::new(config).expect("net");
    assert_eq!(net.check_at(Provider::GitHub, &server.url("/rate_limit")).expect("a verdict"), Verdict::Rejected);
    assert_eq!(net.host_token_state(HOST), TokenState::Rejected);

    // With no token to blame, a 401 is the service wanting a login.
    let err = net.check_at(Provider::GitLab, &server.url("/self")).expect_err("no token any more");
    assert!(matches!(err, NetError::Unauthorized { .. }), "{err:?}");
}

#[test]
fn the_check_reports_a_limit_and_a_server_that_is_down() {
    let limited = TestServer::start(|_, _| {
        Reply::new(403, r#"{"message":"API rate limit exceeded"}"#).header("X-RateLimit-Remaining", "0").header("Retry-After", "600")
    });
    let net = Net::new(local_config()).expect("net");
    assert!(matches!(net.check_at(Provider::GitHub, &limited.url("/rate_limit")), Err(NetError::RateLimited { .. })));
    // A second client: both servers are "127.0.0.1" here, and the first one's remembered wait would answer for the second.
    let down = TestServer::start(|_, _| Reply::new(503, "maintenance"));
    let mut config = local_config();
    config.attempts = 1;
    let fresh = Net::new(config).expect("net");
    assert!(matches!(fresh.check_at(Provider::GitHub, &down.url("/rate_limit")), Err(NetError::Status { status: 503, .. })));
}

#[test]
fn a_gitlab_check_with_a_good_token_is_accepted() {
    let server = TestServer::start(|req, _| {
        assert_eq!(req.header("authorization"), Some("Bearer glpat-check-token-0007"));
        assert!(req.header("x-github-api-version").is_none(), "GitHub's header is for GitHub");
        Reply::ok(r#"{"id":1,"active":true,"revoked":false,"expires_at":"2026-12-31","scopes":["read_api"]}"#)
    });
    let mut config = local_config();
    config.tokens = vec![(HOST.into(), Secret::new("glpat-check-token-0007"))];
    let net = Net::new(config).expect("net");
    let Verdict::Accepted(info) = net.check_at(Provider::GitLab, &server.url("/api/v4/personal_access_tokens/self")).expect("checked")
    else {
        panic!("accepted")
    };
    assert_eq!((info.had_token, info.expires.as_deref()), (true, Some("2026-12-31")));
}
