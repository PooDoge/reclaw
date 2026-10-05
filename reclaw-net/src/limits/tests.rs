use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

use super::*;

fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (k, v) in pairs {
        map.insert(HeaderName::from_bytes(k.as_bytes()).expect("name"), HeaderValue::from_str(v).expect("value"));
    }
    map
}

fn at(secs: u64) -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(secs)
}

#[test]
fn retry_after_is_seconds_or_a_date() {
    assert_eq!(retry_after(&headers(&[("retry-after", "120")]), at(0)), Some(Duration::from_secs(120)));
    let date = "Thu, 01 Jan 1970 00:10:00 GMT";
    assert_eq!(retry_after(&headers(&[("retry-after", date)]), at(0)), Some(Duration::from_secs(600)));
    assert_eq!(retry_after(&headers(&[("retry-after", date)]), at(900)), Some(Duration::ZERO), "a date in the past is no wait");
    assert_eq!(retry_after(&headers(&[("retry-after", "soon")]), at(0)), None);
    assert_eq!(retry_after(&headers(&[]), at(0)), None);
}

#[test]
fn both_services_report_the_window_the_same_way() {
    let github = headers(&[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "1000")]);
    assert_eq!(remaining(&github), Some(0));
    assert_eq!(reset_in(&github, at(400)), Some(Duration::from_secs(600)));
    let gitlab = headers(&[("ratelimit-remaining", "476"), ("ratelimit-reset", "1791158460")]);
    assert_eq!(remaining(&gitlab), Some(476));
    assert_eq!(reset_in(&gitlab, at(1791158400)), Some(Duration::from_secs(60)));
    assert_eq!(reset_in(&github, at(5000)), Some(Duration::ZERO), "a reset in the past is now");
}

#[test]
fn a_429_is_always_a_limit_and_waits_as_told() {
    assert_eq!(limited_for(429, &headers(&[("retry-after", "7")]), "", at(0)), Some(Duration::from_secs(7)));
    assert_eq!(limited_for(429, &headers(&[]), "", at(0)), Some(DEFAULT_WAIT), "no hint, a minute");
}

#[test]
fn a_403_is_a_limit_only_when_something_says_so() {
    let spent = headers(&[("x-ratelimit-remaining", "0"), ("x-ratelimit-reset", "130")]);
    assert_eq!(limited_for(403, &spent, "", at(100)), Some(Duration::from_secs(30)));
    assert_eq!(limited_for(403, &headers(&[("retry-after", "5")]), "", at(0)), Some(Duration::from_secs(5)));
    assert_eq!(limited_for(403, &headers(&[]), "You have exceeded a secondary Rate Limit", at(0)), Some(DEFAULT_WAIT));
    // A permission failure with requests to spare is not a limit.
    let plenty = headers(&[("x-ratelimit-remaining", "59")]);
    assert_eq!(limited_for(403, &plenty, "Resource not accessible", at(0)), None);
    assert_eq!(limited_for(403, &headers(&[]), "", at(0)), None);
}

#[test]
fn a_503_is_a_limit_only_with_a_retry_after() {
    assert_eq!(limited_for(503, &headers(&[("retry-after", "3")]), "", at(0)), Some(Duration::from_secs(3)));
    assert_eq!(limited_for(503, &headers(&[]), "", at(0)), None);
    assert_eq!(limited_for(500, &headers(&[("retry-after", "3")]), "", at(0)), None);
    assert_eq!(limited_for(200, &headers(&[]), "rate limit", at(0)), None);
}

#[test]
fn an_absurd_wait_is_capped() {
    assert_eq!(limited_for(429, &headers(&[("retry-after", "999999")]), "", at(0)), Some(MAX_WAIT));
}
