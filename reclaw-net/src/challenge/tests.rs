use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::*;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        map.insert(HeaderName::from_bytes(k.as_bytes()).expect("name"), HeaderValue::from_str(v).expect("value"));
    }
    map
}

#[test]
fn cloudflares_own_marker_is_enough() {
    assert!(is_bot_challenge(403, &headers(&[("cf-mitigated", "challenge")]), b""));
    assert!(is_bot_challenge(429, &headers(&[("cf-mitigated", "Challenge")]), b"{}"));
}

#[test]
fn an_interstitial_page_is_recognised_by_its_text_on_the_statuses_it_comes_with() {
    let page = b"<!DOCTYPE html><html><head><title>Just a moment...</title></head><body>Checking your browser</body></html>";
    assert!(is_bot_challenge(403, &headers(&[("server", "cloudflare")]), page));
    assert!(is_bot_challenge(503, &headers(&[]), page));
    assert!(!is_bot_challenge(200, &headers(&[]), page), "a 200 is data, whatever it says");
    assert!(!is_bot_challenge(404, &headers(&[]), page));
}

#[test]
fn a_json_error_that_mentions_none_of_this_is_not_a_challenge() {
    assert!(!is_bot_challenge(403, &headers(&[("server", "cloudflare")]), br#"{"message":"403 Forbidden"}"#));
    assert!(!is_bot_challenge(403, &headers(&[]), br#"{"message":"captcha required"}"#), "not an html page");
    assert!(!is_bot_challenge(403, &headers(&[]), b""));
}
