use super::*;

fn accepted(had_token: bool, remaining: u64, limit: u64, extra: &[&str]) -> TokenCheck {
    TokenCheck::Accepted {
        had_token,
        quota: Some(Quota { limit, remaining, reset_at: 0 }),
        expires: None,
        extra_permissions: extra.iter().map(|s| s.to_string()).collect(),
    }
}

fn status(source: TokenSource, check: TokenCheck) -> TokenStatus {
    TokenStatus { source, check }
}

#[test]
fn digits_are_grouped() {
    for (n, text) in [(0, "0"), (60, "60"), (999, "999"), (1000, "1,000"), (4987, "4,987"), (15000, "15,000"), (1_234_567, "1,234,567")] {
        assert_eq!(group_digits(n), text);
    }
}

#[test]
fn with_no_token_github_says_what_the_allowance_is_and_gitlab_says_nothing_more() {
    let none = TokenStatus::default();
    assert_eq!(none.short(Provider::GitHub), "No token (60 requests an hour)");
    assert_eq!(none.short(Provider::GitLab), "No token");
    assert!(!none.has_token());
    let asked = status(TokenSource::None, accepted(false, 58, 60, &[]));
    assert_eq!(asked.short(Provider::GitHub), "No token (58 of 60 left)", "the anonymous allowance, as GitHub states it");
}

#[test]
fn a_token_shows_where_it_is_from_until_it_has_been_checked() {
    assert_eq!(status(TokenSource::Saved, TokenCheck::Unchecked).short(Provider::GitHub), "Saved");
    assert_eq!(status(TokenSource::Environment("GITHUB_TOKEN".into()), TokenCheck::Unchecked).short(Provider::GitHub), "From GITHUB_TOKEN");
    assert!(status(TokenSource::Saved, TokenCheck::Unchecked).has_token());
}

#[test]
fn an_accepted_token_shows_what_is_left() {
    let s = status(TokenSource::Saved, accepted(true, 4987, 5000, &[]));
    assert_eq!(s.short(Provider::GitHub), "4,987 of 5,000 left");
    let broad = status(TokenSource::Saved, accepted(true, 4987, 5000, &["repo"]));
    assert_eq!(broad.short(Provider::GitHub), "4,987 of 5,000 left, has extra permissions");
    let unlimited =
        status(TokenSource::Saved, TokenCheck::Accepted { had_token: true, quota: None, expires: None, extra_permissions: vec![] });
    assert_eq!(unlimited.short(Provider::GitLab), "Accepted");
}

#[test]
fn trouble_is_named_by_the_service_that_caused_it() {
    assert_eq!(status(TokenSource::Saved, TokenCheck::Rejected).short(Provider::GitHub), "GitHub refused it");
    assert_eq!(status(TokenSource::Saved, TokenCheck::Rejected).short(Provider::GitLab), "GitLab refused it");
    assert_eq!(status(TokenSource::Saved, TokenCheck::Failed).short(Provider::GitHub), "Could not check");
    assert_eq!(status(TokenSource::Saved, TokenCheck::Checking).short(Provider::GitHub), "Checking...");
}

#[test]
fn the_two_providers_are_kept_apart() {
    let mut all = CredentialsStatus::default();
    all.set(Provider::GitLab, status(TokenSource::Saved, TokenCheck::Rejected));
    assert_eq!(all.of(Provider::GitLab).check, TokenCheck::Rejected);
    assert_eq!(all.of(Provider::GitHub), &TokenStatus::default());
}
