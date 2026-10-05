use super::*;
use crate::secret::{clear_registered, register_secret};

// The registry is process-wide; tests that touch it take this lock so they do not see each other's secrets.
static REGISTRY: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    let guard = REGISTRY.lock().unwrap_or_else(|e| e.into_inner());
    clear_registered();
    guard
}

const CLASSIC: &str = "ghp_0123456789abcdefghijklmnopqrstuvwxyzAB";
const FINE: &str = "github_pat_11ABCDEFG0abcdefghij_klmnopqrstuvwxyz0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789ab";
const GITLAB: &str = "glpat-aBcDeFgHiJkLmNoPqRsT";

#[test]
fn the_known_token_shapes_are_removed_wherever_they_appear() {
    let _g = lock();
    for token in [CLASSIC, FINE, GITLAB] {
        let line = format!("request to api failed: bad credentials {token} (401)");
        let clean = scrub(&line);
        assert!(!clean.contains(token), "{clean}");
        assert!(clean.contains(REDACTED) && clean.contains("(401)"), "the rest of the line is kept: {clean}");
    }
}

#[test]
fn a_word_that_only_starts_like_a_token_is_left_alone() {
    let _g = lock();
    for text in ["ghp_ is the prefix", "see ghp_short", "the glpat-x form", "pushing to origin"] {
        assert_eq!(scrub(text), text);
    }
}

#[test]
fn a_prefix_inside_a_longer_word_is_not_a_token() {
    let _g = lock();
    let text = "xghp_0123456789abcdefghijklmnop";
    assert_eq!(scrub(text), text, "a token starts at a boundary");
}

#[test]
fn registered_values_go_in_any_shape() {
    let _g = lock();
    register_secret("corp-token-7731-xyz");
    let clean = scrub("sent corp-token-7731-xyz to host; again corp-token-7731-xyz");
    assert_eq!(clean, format!("sent {REDACTED} to host; again {REDACTED}"));
}

#[test]
fn a_short_value_is_never_registered() {
    let _g = lock();
    register_secret("abc");
    assert_eq!(scrub("abc abc abc"), "abc abc abc");
}

#[test]
fn a_registered_value_that_contains_another_is_replaced_whole() {
    let _g = lock();
    register_secret("secret-value-1");
    register_secret("secret-value-1-and-more");
    assert_eq!(scrub("x secret-value-1-and-more y"), format!("x {REDACTED} y"));
}

#[test]
fn credential_headers_lose_their_value_and_keep_their_name() {
    let _g = lock();
    assert_eq!(scrub("Authorization: Bearer abc.def.ghi\nnext line"), format!("Authorization: {REDACTED}\nnext line"));
    assert_eq!(scrub("PRIVATE-TOKEN: whatever-it-is"), format!("PRIVATE-TOKEN: {REDACTED}"));
    assert_eq!(scrub("sent Bearer abcd1234 and went on"), format!("sent Bearer {REDACTED} and went on"));
}

#[test]
fn an_address_loses_its_user_and_password_and_keeps_the_host() {
    let _g = lock();
    let clean = scrub("fetching https://alice:hunter2@example.org/a/b?x=1 failed");
    assert_eq!(clean, format!("fetching https://{REDACTED}@example.org/a/b?x=1 failed"));
}

#[test]
fn query_credentials_lose_the_value_only() {
    let _g = lock();
    assert_eq!(
        scrub("GET https://x.test/f?name=1&access_token=abc123&other=2 done"),
        format!("GET https://x.test/f?name=1&access_token={REDACTED}&other=2 done")
    );
    // Not a query parameter: prose.
    assert_eq!(scrub("the token=expired state"), "the token=expired state");
}

#[test]
fn clean_text_is_borrowed_not_copied() {
    let _g = lock();
    assert!(matches!(scrub("2026-10-05T10:00:00Z INFO reclaw_net: fetched catalog in 443 ms"), Cow::Borrowed(_)));
}

#[test]
fn scrubbing_twice_changes_nothing_and_odd_text_does_not_panic() {
    let _g = lock();
    for text in [
        format!("{CLASSIC} Authorization: x https://u:p@h/ ?token=zzz"),
        "ünïcödé ghp_ñññññññññññññññññ ‹ ›".to_string(),
        "Bearer ".to_string(),
        "://@".to_string(),
        "?token=".to_string(),
        String::new(),
    ] {
        let once = scrub(&text).into_owned();
        assert_eq!(scrub(&once), once, "idempotent for {text:?}");
    }
}

#[test]
fn a_secret_prints_as_nothing() {
    let secret = Secret::new(CLASSIC);
    assert_eq!(format!("{secret}"), REDACTED);
    assert!(!format!("{secret:?}").contains("ghp_"));
    assert_eq!(secret.expose(), CLASSIC);
    let wrapped = format!("{:?}", vec![Some(secret)]);
    assert!(!wrapped.contains("0123456789"), "{wrapped}");
}

use crate::secret::Secret;
