use super::*;

fn creds() -> Credentials {
    Credentials::new(&[
        ("api.github.com".into(), Secret::new("ghp_verysecrettoken0000")),
        ("gitlab.com".into(), Secret::new("glpat-another-secret-1")),
    ])
}

#[test]
fn each_provider_has_one_host_that_gets_its_token() {
    assert_eq!(Provider::GitHub.host(), "api.github.com");
    assert_eq!(Provider::from_host("GITLAB.com"), Some(Provider::GitLab));
    assert_eq!(Provider::from_host("github.com"), None, "the website is not the API");
    for p in Provider::ALL {
        assert!(p.token_page().starts_with("https://") && p.check_url().starts_with("https://"));
        assert!(p.check_url().contains(p.host()), "the check goes to the host that gets the token");
    }
}

#[test]
fn the_token_is_a_bearer_header_marked_sensitive_for_both_services() {
    let c = creds();
    let (name, value) = c.header("api.github.com").expect("github");
    assert_eq!((name, value.to_str().expect("text"), value.is_sensitive()), (AUTHORIZATION, "Bearer ghp_verysecrettoken0000", true));
    let (name, value) = c.header("GitLab.com").expect("gitlab");
    assert_eq!(
        (name, value.to_str().expect("text")),
        (AUTHORIZATION, "Bearer glpat-another-secret-1"),
        "the one header the client strips on a redirect"
    );
    assert!(c.header("raw.githubusercontent.com").is_none() && c.header("github.com").is_none(), "only the API host");
}

#[test]
fn a_rejected_token_is_kept_but_no_longer_sent_and_a_new_one_is_trusted_again() {
    let c = creds();
    assert!(c.reject("api.github.com"), "news the first time");
    assert!(!c.reject("api.github.com"), "not news the second");
    assert_eq!(c.state("api.github.com"), TokenState::Rejected);
    assert!(c.header("api.github.com").is_none());
    assert_eq!(c.tag("api.github.com"), "anon", "an unsent token does not make a separate identity");
    assert_eq!(c.state("gitlab.com"), TokenState::Active, "the other service is unaffected");
    c.set("api.github.com", Some(Secret::new("ghp_replacement0000000")));
    assert_eq!(c.state("api.github.com"), TokenState::Active);
    assert!(c.header("api.github.com").is_some());
}

#[test]
fn removing_a_token_leaves_none() {
    let c = creds();
    c.set("api.github.com", None);
    assert_eq!(c.state("api.github.com"), TokenState::None);
    assert!(!c.reject("api.github.com"), "nothing to reject");
    assert_eq!(c.tag("api.github.com"), "anon");
}

#[test]
fn the_cache_identity_changes_with_the_token_and_reveals_nothing() {
    let c = creds();
    let first = c.tag("api.github.com");
    assert_eq!(first.len(), 12);
    assert!(!first.contains("verysecret"));
    assert_eq!(c.tag("api.github.com"), first, "stable");
    c.set("api.github.com", Some(Secret::new("ghp_a_different_token_0000")));
    assert_ne!(c.tag("api.github.com"), first);
    assert_eq!(c.tag("example.org"), "anon");
}

#[test]
fn a_value_a_header_cannot_hold_sends_nothing() {
    let c = Credentials::new(&[("api.github.com".into(), Secret::new("bad\ntoken-value-0000"))]);
    assert!(c.header("api.github.com").is_none());
}

#[test]
fn the_same_token_set_twice_stays_covered_by_the_logs_redaction() {
    let c = Credentials::new(&[]);
    let token = "corp-credential-set-twice-0001";
    c.set("api.github.com", Some(Secret::new(token)));
    c.set("api.github.com", Some(Secret::new(token)));
    assert_eq!(
        reclaw_log::scrub(&format!("x {token} y")),
        format!("x {} y", reclaw_log::REDACTED),
        "applied again at start, still redacted"
    );
    c.set("api.github.com", Some(Secret::new("corp-credential-replacement-0002")));
    assert_eq!(reclaw_log::scrub(&format!("x {token} y")), format!("x {token} y"), "a replaced token is forgotten");
}

#[test]
fn a_token_is_registered_for_redaction_while_it_is_held_and_forgotten_after() {
    let c = Credentials::new(&[]);
    c.set("api.github.com", Some(Secret::new("corp-credential-in-no-known-shape")));
    assert_eq!(reclaw_log::scrub("x corp-credential-in-no-known-shape y"), format!("x {} y", reclaw_log::REDACTED));
    c.set("api.github.com", None);
    assert_eq!(reclaw_log::scrub("x corp-credential-in-no-known-shape y"), "x corp-credential-in-no-known-shape y");
}
