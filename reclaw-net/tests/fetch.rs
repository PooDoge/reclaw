//! `Net::fetch` against a server on this machine: what it asks, what it does with each kind of answer, and what the
//! cache does when the answer is old, unchanged or unreachable.
use std::{
    io::Write,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use reclaw_net::testing::{Reply, TestServer, local_config, local_net};
use reclaw_net::{AddressPolicy, Net, NetConfig, NetError, ProxyMode, Request, Source, UrlError};

#[test]
fn a_body_arrives_with_its_headers_and_the_request_says_who_is_asking() {
    let server = TestServer::start(|_, _| Reply::ok(r#"{"a":1}"#).header("ETag", "\"v1\"").header("Content-Type", "application/json"));
    let got =
        local_net().fetch(&Request::get(server.url("/x")).accept("application/vnd.github+json").header("X-Test", "yes")).expect("fetched");
    assert_eq!((got.status, got.text().as_str(), got.source), (200, r#"{"a":1}"#, Source::Network));
    assert_eq!((got.etag.as_deref(), got.content_type.as_deref()), (Some("\"v1\""), Some("application/json")));
    let req = &server.requests()[0];
    assert!(req.header("user-agent").is_some_and(|ua| ua.starts_with("Reclaw/") && ua.contains("github.com/poodoge/reclaw")), "{req:?}");
    assert_eq!(req.header("accept"), Some("application/vnd.github+json"));
    assert_eq!(req.header("x-test"), Some("yes"));
    assert!(req.header("authorization").is_none(), "no credentials unless configured for the host");
}

#[test]
fn a_gzipped_answer_is_unpacked() {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(b"hello hello hello hello").expect("gzip");
    let packed = encoder.finish().expect("finish");
    let server = TestServer::start(move |req, _| {
        assert!(req.header("accept-encoding").is_some_and(|e| e.contains("gzip")), "advertises what it can unpack");
        Reply::ok(packed.clone()).header("Content-Encoding", "gzip")
    });
    assert_eq!(local_net().fetch(&Request::get(server.url("/z"))).expect("fetched").text(), "hello hello hello hello");
}

#[test]
fn a_server_fault_is_retried_and_a_missing_page_is_not() {
    let server = TestServer::start(|_, n| if n < 2 { Reply::new(503, "busy") } else { Reply::ok("fine") });
    let mut config = local_config();
    config.attempts = 3;
    let got = Net::new(config).expect("net").fetch(&Request::get(server.url("/"))).expect("third time lucky");
    assert_eq!((got.text().as_str(), server.count()), ("fine", 3));

    let missing = TestServer::start(|_, _| Reply::new(404, "nope"));
    assert_eq!(local_net().fetch(&Request::get(missing.url("/"))), Err(NetError::Status { host: "127.0.0.1".into(), status: 404 }));
    assert_eq!(missing.count(), 1, "a 404 is an answer");
}

#[test]
fn persistent_trouble_stops_after_the_configured_attempts() {
    let server = TestServer::start(|_, _| Reply::new(503, "busy"));
    let mut config = local_config();
    config.attempts = 2;
    let err = Net::new(config).expect("net").fetch(&Request::get(server.url("/"))).expect_err("gives up");
    assert!(matches!(err, NetError::Status { status: 503, .. }), "{err:?}");
    assert_eq!(server.count(), 2);
}

#[test]
fn a_brief_rate_limit_is_waited_out_and_a_long_one_is_remembered() {
    let server = TestServer::start(|_, n| if n == 0 { Reply::new(429, "slow down").header("Retry-After", "0") } else { Reply::ok("ok") });
    assert_eq!(local_net().fetch(&Request::get(server.url("/"))).expect("after the pause").text(), "ok");

    let limited = TestServer::start(|_, _| Reply::new(429, "slow down").header("Retry-After", "120"));
    let net = local_net();
    let err = net.fetch(&Request::get(limited.url("/a"))).expect_err("a long wait is an error");
    assert!(matches!(&err, NetError::RateLimited { retry_in, .. } if retry_in.as_secs() == 120), "{err:?}");
    assert!(err.hint().is_some());
    let again = net.fetch(&Request::get(limited.url("/b"))).expect_err("still limited");
    assert!(matches!(again, NetError::RateLimited { .. }));
    assert_eq!(limited.count(), 1, "the second request was not even sent");
}

#[test]
fn github_style_limits_are_recognised_from_the_headers() {
    let server = TestServer::start(|_, _| {
        Reply::new(403, r#"{"message":"API rate limit exceeded"}"#)
            .header("X-RateLimit-Remaining", "0")
            .header("X-RateLimit-Reset", "9999999999")
    });
    let err = local_net().fetch(&Request::get(server.url("/"))).expect_err("limited");
    assert!(matches!(err, NetError::RateLimited { .. }), "{err:?}");

    let forbidden =
        TestServer::start(|_, _| Reply::new(403, r#"{"message":"Resource not accessible"}"#).header("X-RateLimit-Remaining", "55"));
    assert!(matches!(local_net().fetch(&Request::get(forbidden.url("/"))), Err(NetError::Status { status: 403, .. })));
}

#[test]
fn a_bot_check_page_is_named_and_not_retried() {
    let server = TestServer::start(|_, _| {
        Reply::new(403, "<!DOCTYPE html><title>Just a moment...</title>").header("cf-mitigated", "challenge").header("Server", "cloudflare")
    });
    let err = local_net().fetch(&Request::get(server.url("/"))).expect_err("challenge");
    assert!(matches!(err, NetError::BotChallenge { status: 403, .. }), "{err:?}");
    assert!(err.hint().is_some_and(|h| h.contains("does not imitate")));
    assert_eq!(server.count(), 1);
}

#[test]
fn a_body_over_the_limit_is_refused_whether_announced_or_not() {
    let big = vec![b'x'; 10_000];
    let announced = TestServer::start({
        let big = big.clone();
        move |_, _| Reply::ok(big.clone())
    });
    assert_eq!(local_net().fetch(&Request::get(announced.url("/")).max_bytes(1000)), Err(NetError::TooLarge { limit: 1000 }));
    let unannounced = TestServer::start(move |_, _| {
        let mut r = Reply::ok(big.clone());
        r.unframed = true;
        r
    });
    assert_eq!(local_net().fetch(&Request::get(unannounced.url("/")).max_bytes(1000)), Err(NetError::TooLarge { limit: 1000 }));
}

#[test]
fn a_slow_server_times_out() {
    let server = TestServer::start(|_, _| {
        let mut r = Reply::ok("late");
        r.delay = Duration::from_secs(3);
        r
    });
    let mut config = local_config();
    config.attempts = 1;
    let err =
        Net::new(config).expect("net").fetch(&Request::get(server.url("/")).timeout(Duration::from_millis(300))).expect_err("times out");
    assert_eq!(err, NetError::Timeout);
}

#[test]
fn a_fresh_copy_is_used_without_asking_and_a_stale_one_is_revalidated() {
    let dir = tempfile::tempdir().expect("tempdir");
    let hits = std::sync::Arc::new(AtomicUsize::new(0));
    let server = TestServer::start({
        let hits = hits.clone();
        move |req, _| {
            hits.fetch_add(1, Ordering::SeqCst);
            if req.header("if-none-match") == Some("\"v1\"") { Reply::new(304, "") } else { Reply::ok("body-v1").header("ETag", "\"v1\"") }
        }
    });
    let mut config = local_config();
    config.cache_dir = Some(dir.path().to_path_buf());
    let net = Net::new(config).expect("net");

    let first = net.fetch(&Request::get(server.url("/c")).cached(Duration::from_secs(3600), false)).expect("first");
    assert_eq!((first.source, first.text().as_str()), (Source::Network, "body-v1"));
    let second = net.fetch(&Request::get(server.url("/c")).cached(Duration::from_secs(3600), false)).expect("second");
    assert_eq!((second.source, second.text().as_str()), (Source::CacheFresh, "body-v1"));
    assert_eq!(hits.load(Ordering::SeqCst), 1, "nothing was asked the second time");

    // A time to live of zero makes every copy old: the server is asked, and says nothing changed.
    let third = net.fetch(&Request::get(server.url("/c")).cached(Duration::ZERO, false)).expect("third");
    assert_eq!((third.source, third.text().as_str()), (Source::CacheRevalidated, "body-v1"));
    assert_eq!(hits.load(Ordering::SeqCst), 2);
    assert_eq!(server.requests()[1].header("if-none-match"), Some("\"v1\""));
}

#[test]
fn when_the_network_is_down_an_old_copy_is_better_than_an_error_if_allowed() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut config = local_config();
    config.cache_dir = Some(dir.path().to_path_buf());
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    let url = {
        let server = TestServer::start(|_, _| Reply::ok("saved").header("ETag", "\"s\""));
        let url = server.url("/d");
        net.fetch(&Request::get(url.clone()).cached(Duration::from_secs(60), true)).expect("online");
        url
    };
    // The server is gone now.
    std::thread::sleep(Duration::from_millis(100));
    let stale = net.fetch(&Request::get(url.clone()).cached(Duration::ZERO, true)).expect("served from the cache");
    assert_eq!((stale.source, stale.text().as_str()), (Source::CacheStale, "saved"));
    assert!(stale.is_stale() && stale.stale_because.as_ref().is_some_and(NetError::is_connectivity), "{:?}", stale.stale_because);
    let strict = net.fetch(&Request::get(url).cached(Duration::ZERO, false));
    assert!(strict.is_err(), "without permission the failure is shown");
}

#[test]
fn a_token_goes_to_its_own_host_and_is_dropped_when_a_redirect_leaves_it() {
    let server = TestServer::start(|req, _| {
        if req.path == "/start" {
            // The same server under another name: a different host as far as credentials are concerned.
            let port = req.header("host").and_then(|h| h.rsplit(':').next()).unwrap_or("0").to_string();
            Reply::new(302, "").header("Location", &format!("http://localhost:{port}/final"))
        } else {
            Reply::ok("done")
        }
    });
    let mut config = local_config();
    config.tokens = vec![("127.0.0.1".into(), "s3cret".into())];
    let net = Net::new(config).expect("net");
    net.fetch(&Request::get(server.url("/start"))).expect("followed");
    let seen = server.requests();
    assert_eq!(seen[0].header("authorization"), Some("Bearer s3cret"));
    assert_eq!(seen[1].path, "/final");
    assert!(seen[1].header("authorization").is_none(), "the credential must not follow a redirect to another host: {:?}", seen[1]);
}

#[test]
fn an_authenticated_answer_is_not_served_to_an_anonymous_request() {
    let dir = tempfile::tempdir().expect("tempdir");
    let server = TestServer::start(|req, _| if req.header("authorization").is_some() { Reply::ok("private") } else { Reply::ok("public") });
    let rule = |r: Request| r.cached(Duration::from_secs(3600), false);
    let mut with = local_config();
    with.cache_dir = Some(dir.path().to_path_buf());
    with.tokens = vec![("127.0.0.1".into(), "tok".into())];
    let mut without = local_config();
    without.cache_dir = Some(dir.path().to_path_buf());
    assert_eq!(Net::new(with).expect("net").fetch(&rule(Request::get(server.url("/who")))).expect("a").text(), "private");
    assert_eq!(
        Net::new(without).expect("net").fetch(&rule(Request::get(server.url("/who")))).expect("b").text(),
        "public",
        "no leak across identities"
    );
}

#[test]
fn the_address_policy_stops_a_request_before_anything_is_sent() {
    let net = Net::new(NetConfig { proxy: ProxyMode::None, ..NetConfig::default() }).expect("net");
    for (url, why) in [
        ("https://127.0.0.1/x", UrlError::LocalNetwork),
        ("https://192.168.1.1/x", UrlError::LocalNetwork),
        ("http://example.com/x", UrlError::NotHttps),
        ("https://user:pw@example.com/x", UrlError::Credentials),
        ("https://example.com:8443/x", UrlError::Port),
    ] {
        assert_eq!(net.fetch(&Request::get(url)), Err(NetError::Blocked(why)), "{url}");
    }
    assert_eq!(AddressPolicy::default(), AddressPolicy::Public, "the safe policy is the default");
}

#[test]
fn a_proxy_that_refuses_the_tunnel_is_reported_as_the_networks_decision() {
    let proxy = TestServer::start(|req, _| if req.method == "CONNECT" { Reply::new(403, "denied by policy") } else { Reply::new(502, "") });
    let net =
        Net::new(NetConfig { proxy: ProxyMode::Url(format!("http://{}", proxy.addr)), attempts: 3, ..NetConfig::default() }).expect("net");
    let err = net.fetch(&Request::get("https://example.com/thing")).expect_err("refused");
    assert!(matches!(err, NetError::ProxyDenied { ref host, .. } if host == "example.com"), "{err:?}");
    assert_eq!(proxy.count(), 1, "a policy refusal is not retried");
    assert!(err.hint().is_some_and(|h| h.contains("does not route around")));
}

#[test]
fn a_refused_host_is_not_asked_again_for_every_picture() {
    let proxy = TestServer::start(|req, _| if req.method == "CONNECT" { Reply::new(403, "denied by policy") } else { Reply::new(502, "") });
    let net =
        Net::new(NetConfig { proxy: ProxyMode::Url(format!("http://{}", proxy.addr)), attempts: 3, ..NetConfig::default() }).expect("net");
    let first = net.fetch(&Request::get("https://cdn.example.com/icon-1.png")).expect_err("refused");
    assert!(matches!(first, NetError::ProxyDenied { .. }));
    for i in 2..=20 {
        let again = net.fetch(&Request::get(format!("https://cdn.example.com/icon-{i}.png"))).expect_err("still refused");
        assert_eq!(again, first, "the same answer, from memory");
    }
    assert_eq!(proxy.count(), 1, "the network was asked once, not twenty times");
    let other = net.fetch(&Request::get("https://other.example.com/x")).expect_err("also refused, but asked");
    assert!(matches!(other, NetError::ProxyDenied { ref host, .. } if host == "other.example.com"));
    assert_eq!(proxy.count(), 2, "another host gets its own question");
}

#[test]
fn nothing_listening_and_no_such_name_are_told_apart() {
    let mut config = local_config();
    config.attempts = 1;
    let net = Net::new(config).expect("net");
    let refused = net.fetch(&Request::get("http://127.0.0.1:1/x")).expect_err("refused");
    assert!(matches!(refused, NetError::Connect { .. }), "{refused:?}");
    let unknown = net.fetch(&Request::get("http://no-such-host.invalid/x")).expect_err("unknown");
    assert!(matches!(unknown, NetError::Dns { .. } | NetError::Connect { .. }), "{unknown:?}");
}

#[tokio::test]
async fn a_blocking_call_from_async_code_is_an_error_not_a_deadlock() {
    let net = local_net();
    assert_eq!(net.fetch(&Request::get("http://127.0.0.1:1/x")), Err(NetError::WrongContext));
}

#[test]
fn several_requests_at_once_share_the_work() {
    let server = TestServer::start(|req, _| Reply::ok(req.path.clone()));
    let net = local_net();
    let handles: Vec<_> = (0..12)
        .map(|i| {
            let (net, url) = (net.clone(), server.url(&format!("/{i}")));
            std::thread::spawn(move || net.fetch(&Request::get(url)).map(|f| f.text()))
        })
        .collect();
    let mut got: Vec<String> = handles.into_iter().map(|h| h.join().expect("thread").expect("fetched")).collect();
    got.sort();
    let mut want: Vec<String> = (0..12).map(|i| format!("/{i}")).collect();
    want.sort();
    assert_eq!(got, want);
}
