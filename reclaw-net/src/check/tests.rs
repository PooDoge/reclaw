use reqwest::header::{HeaderName, HeaderValue};

use super::*;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        map.insert(HeaderName::from_bytes(k.as_bytes()).expect("name"), HeaderValue::from_str(v).expect("value"));
    }
    map
}

const GITHUB_BODY: &[u8] = br#"{"resources":{"core":{"limit":5000,"used":13,"remaining":4987,"reset":1790000000},"search":{"limit":30,"used":0,"remaining":30,"reset":1790000060}},"rate":{"limit":5000,"used":13,"remaining":4987,"reset":1790000000}}"#;

#[test]
fn github_core_quota_is_read_from_the_body() {
    let info = github_info(&headers(&[]), GITHUB_BODY, true);
    assert_eq!(info.quota, Some(Quota { limit: 5000, remaining: 4987, reset_at: 1_790_000_000 }));
    assert!(info.had_token);
}

#[test]
fn the_headers_stand_in_when_the_body_has_no_quota() {
    let info = github_info(
        &headers(&[("x-ratelimit-limit", "60"), ("x-ratelimit-remaining", "58"), ("x-ratelimit-reset", "1790000000")]),
        b"{}",
        false,
    );
    assert_eq!(info.quota, Some(Quota { limit: 60, remaining: 58, reset_at: 1_790_000_000 }));
    assert!(!info.had_token);
}

#[test]
fn classic_token_scopes_and_expiry_are_reported_and_an_empty_scope_list_is_empty_not_missing() {
    let none = github_info(
        &headers(&[("x-oauth-scopes", ""), ("github-authentication-token-expiration", "2026-12-31 00:00:00 UTC")]),
        GITHUB_BODY,
        true,
    );
    assert_eq!(none.scopes, Some(vec![]), "a token with no permissions: exactly what Reclaw wants");
    assert_eq!(none.expires.as_deref(), Some("2026-12-31 00:00:00 UTC"));
    let broad = github_info(&headers(&[("x-oauth-scopes", "repo, read:org")]), GITHUB_BODY, true);
    assert_eq!(broad.scopes, Some(vec!["repo".to_string(), "read:org".to_string()]));
    let fine_grained = github_info(&headers(&[]), GITHUB_BODY, true);
    assert_eq!(fine_grained.scopes, None, "no header, no claim");
    let anonymous = github_info(&headers(&[("x-oauth-scopes", "repo")]), GITHUB_BODY, false);
    assert_eq!(anonymous.scopes, None, "an anonymous client has no scopes to report");
}

#[test]
fn gitlab_reports_expiry_and_a_revoked_or_inactive_token_is_rejected() {
    let ok = gitlab_verdict(
        &headers(&[("ratelimit-limit", "2000"), ("ratelimit-remaining", "1999"), ("ratelimit-reset", "1790000000")]),
        br#"{"active":true,"revoked":false,"expires_at":"2026-12-31","scopes":["read_api"]}"#,
    );
    let Verdict::Accepted(info) = ok else { panic!("accepted: {ok:?}") };
    assert_eq!((info.expires.as_deref(), info.quota.map(|q| q.limit)), (Some("2026-12-31"), Some(2000)));
    assert_eq!(gitlab_verdict(&headers(&[]), br#"{"active":false}"#), Verdict::Rejected);
    assert_eq!(gitlab_verdict(&headers(&[]), br#"{"revoked":true}"#), Verdict::Rejected);
    assert!(
        matches!(gitlab_verdict(&headers(&[]), b"not json"), Verdict::Accepted(_)),
        "a 200 is acceptance even in a shape we do not know"
    );
}
