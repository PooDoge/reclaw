use std::collections::HashMap;

use super::*;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let map: HashMap<String, String> = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    move |k| map.get(k).cloned()
}

#[test]
fn the_default_identifies_the_program_and_where_to_find_it() {
    let ua = NetConfig::default().user_agent;
    assert!(ua.starts_with("Reclaw/") && ua.contains(PROJECT_URL), "{ua}");
}

#[test]
fn tokens_come_from_the_environment_for_their_own_host_only() {
    let (config, problems) = NetConfig::from_env(env(&[("GITHUB_TOKEN", " ghp_secret "), ("GITLAB_TOKEN", "")]));
    assert!(problems.is_empty());
    assert_eq!(config.token_for("api.github.com"), Some("ghp_secret"));
    assert_eq!(config.token_for("API.GITHUB.COM"), Some("ghp_secret"));
    assert_eq!(config.token_for("github.com"), None, "not sent to the website or the download hosts");
    assert_eq!(config.token_for("gitlab.com"), None, "an empty variable is no token");
    let own = NetConfig::from_env(env(&[("GITHUB_TOKEN", "a"), ("RECLAW_GITHUB_TOKEN", "b")])).0;
    assert_eq!(own.token_for("api.github.com"), Some("b"), "Reclaw's own variable wins");
}

#[test]
fn a_token_never_appears_in_debug_output_or_in_a_cache_key() {
    let config = NetConfig::from_env(env(&[("GITHUB_TOKEN", "ghp_verysecret")])).0;
    assert!(!format!("{config:?}").contains("verysecret"));
    assert!(format!("{config:?}").contains("api.github.com"));
    let tag = config.credential_tag("api.github.com");
    assert!(!tag.contains("verysecret") && tag.len() == 12, "{tag}");
    assert_eq!(config.credential_tag("raw.githubusercontent.com"), "anon");
    assert_eq!(config.credential_tag("api.github.com"), tag, "stable");
}

#[test]
fn proxy_settings_follow_the_variable() {
    assert_eq!(NetConfig::from_env(env(&[])).0.proxy, ProxyMode::System);
    assert_eq!(NetConfig::from_env(env(&[("RECLAW_PROXY", "NONE")])).0.proxy, ProxyMode::None);
    assert_eq!(
        NetConfig::from_env(env(&[("RECLAW_PROXY", "http://127.0.0.1:3128")])).0.proxy,
        ProxyMode::Url("http://127.0.0.1:3128".into())
    );
}

#[test]
fn an_unreadable_certificate_file_is_reported_not_fatal() {
    let (config, problems) = NetConfig::from_env(env(&[("SSL_CERT_FILE", "/nonexistent/ca.pem")]));
    assert!(config.extra_roots_pem.is_none());
    assert_eq!(problems.len(), 1);
    assert!(problems[0].contains("could not be read"));
}

#[test]
fn a_readable_certificate_file_is_loaded() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ca.pem");
    std::fs::write(&path, "pem").expect("write");
    let (config, problems) = NetConfig::from_env(env(&[("SSL_CERT_FILE", path.to_str().expect("utf8"))]));
    assert!(problems.is_empty());
    assert_eq!(config.extra_roots_pem.as_deref(), Some(&b"pem"[..]));
}
