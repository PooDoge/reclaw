use super::*;

fn refused(text: &str) -> UrlError {
    MediaUrl::parse(text).expect_err(text)
}

#[test]
fn ordinary_https_addresses_are_accepted() {
    for ok in [
        "https://raw.githubusercontent.com/owner/repo/HEAD/docs/shot.png",
        "https://img.shields.io/badge/build-passing-green",
        "https://example.com:443/a.png",
        "  https://example.com/a.png  ",
        "https://8.8.8.8/a.png",
    ] {
        assert!(MediaUrl::parse(ok).is_ok(), "{ok}");
    }
    assert_eq!(MediaUrl::parse("https://Example.COM/a.png").expect("ok").host(), "example.com");
}

#[test]
fn other_schemes_are_refused() {
    for (text, why) in [
        ("http://example.com/a.png", UrlError::NotHttps),
        ("file:///etc/passwd", UrlError::NotHttps),
        ("data:image/png;base64,AAAA", UrlError::NotHttps),
        ("javascript:alert(1)", UrlError::NotHttps),
        ("ftp://example.com/a.png", UrlError::NotHttps),
    ] {
        assert_eq!(refused(text), why, "{text}");
    }
    assert!(matches!(refused("not a url"), UrlError::Invalid(_)));
    assert!(matches!(refused(""), UrlError::Invalid(_)));
}

#[test]
fn credentials_and_odd_ports_are_refused() {
    assert_eq!(refused("https://user:pw@example.com/a.png"), UrlError::Credentials);
    assert_eq!(refused("https://user@example.com/a.png"), UrlError::Credentials);
    assert_eq!(refused("https://example.com:8443/a.png"), UrlError::Port);
}

#[test]
fn the_local_network_is_off_limits_by_name() {
    for name in [
        "https://localhost/a.png",
        "https://LOCALHOST./a.png",
        "https://app.localhost/a.png",
        "https://printer/a.png",
        "https://nas.local/a.png",
        "https://router.lan/a.png",
        "https://svc.internal/a.png",
    ] {
        assert_eq!(refused(name), UrlError::LocalNetwork, "{name}");
    }
}

#[test]
fn the_local_network_is_off_limits_by_number() {
    for ip in [
        "https://127.0.0.1/a.png",
        "https://10.1.2.3/a.png",
        "https://192.168.1.1/a.png",
        "https://172.16.0.1/a.png",
        "https://169.254.169.254/latest/meta-data",
        "https://100.64.0.1/a.png",
        "https://0.0.0.0/a.png",
        "https://[::1]/a.png",
        "https://[fe80::1]/a.png",
        "https://[fc00::1]/a.png",
        "https://[::ffff:192.168.0.1]/a.png",
    ] {
        assert_eq!(refused(ip), UrlError::LocalNetwork, "{ip}");
    }
    assert!(MediaUrl::parse("https://[2606:4700:4700::1111]/a.png").is_ok(), "a public IPv6 address is fine");
}

#[test]
fn an_overlong_address_is_refused() {
    let long = format!("https://example.com/{}", "a".repeat(3000));
    assert_eq!(refused(&long), UrlError::TooLong);
}
