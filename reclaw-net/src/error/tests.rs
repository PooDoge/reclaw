use super::*;

fn kind(text: &str) -> NetError {
    classify_text("example.com", text, false, false)
}

#[test]
fn a_refused_tunnel_is_the_networks_policy_not_a_site_error() {
    // The words reqwest/hyper use when a proxy answers CONNECT with an error.
    let e = kind(
        "error sending request for url (https://example.com/): client error (Connect): proxy connect error: unsuccessful tunnel (403 Forbidden)",
    );
    assert_eq!(e, NetError::ProxyDenied { host: "example.com".into(), status: Some(403) });
    assert!(e.hint().is_some_and(|h| h.contains("does not route around")));
    assert!(matches!(kind("proxy authentication required"), NetError::ProxyDenied { status: None, .. }));
}

#[test]
fn certificate_trouble_points_at_the_ca_bundle() {
    for text in ["invalid peer certificate: UnknownIssuer", "error: tls handshake eof", "received fatal alert: HandshakeFailure"] {
        let e = kind(text);
        assert!(matches!(e, NetError::Tls { .. }), "{text} -> {e:?}");
        assert!(e.hint().is_some_and(|h| h.contains("SSL_CERT_FILE")));
    }
}

#[test]
fn dns_timeout_and_connect_failures_are_told_apart() {
    assert!(matches!(kind("dns error: failed to lookup address information: Name or service not known"), NetError::Dns { .. }));
    assert_eq!(classify_text("h", "operation timed out", true, false), NetError::Timeout);
    assert!(matches!(kind("error trying to connect: Connection refused (os error 111)"), NetError::Connect { .. }));
    assert!(matches!(classify_text("h", "something odd", false, true), NetError::Connect { .. }));
    assert!(matches!(kind("something nobody expected"), NetError::Other(_)));
}

#[test]
fn only_failures_that_say_nothing_about_the_request_are_retried() {
    let status = |s| NetError::Status { host: "h".into(), status: s };
    for s in [408, 425, 500, 502, 503, 504] {
        assert!(status(s).is_transient(), "{s}");
    }
    for s in [400, 401, 403, 404, 410, 451] {
        assert!(!status(s).is_transient(), "{s}");
    }
    assert!(NetError::Timeout.is_transient() && NetError::Stalled(Duration::from_secs(20)).is_transient());
    assert!(!NetError::ProxyDenied { host: "h".into(), status: Some(403) }.is_transient(), "policy does not change on retry");
    assert!(!NetError::BotChallenge { host: "h".into(), status: 403 }.is_transient());
    assert!(!NetError::Cancelled.is_transient());
}

#[test]
fn a_saved_copy_is_reasonable_when_the_network_is_down_but_not_when_the_request_was_wrong() {
    let down = [
        NetError::Timeout,
        NetError::Dns { host: "h".into() },
        NetError::ProxyDenied { host: "h".into(), status: None },
        NetError::RateLimited { host: "h".into(), retry_in: Duration::from_secs(60) },
        NetError::Status { host: "h".into(), status: 503 },
    ];
    assert!(down.iter().all(NetError::is_connectivity));
    assert!(!NetError::Status { host: "h".into(), status: 404 }.is_connectivity(), "a 404 is an answer");
    assert!(!NetError::TooLarge { limit: 1 }.is_connectivity());
}

#[test]
fn a_full_disk_is_named_and_other_io_errors_keep_their_words() {
    assert_eq!(from_io(&std::io::Error::from(std::io::ErrorKind::StorageFull)), NetError::DiskFull);
    assert!(matches!(from_io(&std::io::Error::other("boom")), NetError::Io(m) if m.contains("boom")));
}

#[test]
fn the_text_of_a_chain_includes_every_cause() {
    #[derive(Debug)]
    struct Inner;
    impl std::fmt::Display for Inner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "the real cause")
        }
    }
    impl std::error::Error for Inner {}
    #[derive(Debug)]
    struct Outer(Inner);
    impl std::fmt::Display for Outer {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "outer")
        }
    }
    impl std::error::Error for Outer {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            Some(&self.0)
        }
    }
    assert_eq!(chain_text(&Outer(Inner)), "outer: the real cause");
    assert_eq!(first_cause("a: b: c"), "c");
}
