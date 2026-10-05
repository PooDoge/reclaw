//! Against the real services, by hand: `cargo test -p reclaw-net --test live -- --ignored --nocapture`. These spend a few of
//! GitHub's 60 anonymous requests an hour (or of a token's 5,000, when `GITHUB_TOKEN` is set), so they are not part of the normal
//! run. They check the two things the design rests on: the service answers the check the Settings page makes, and a conditional
//! request that is answered `304 Not Modified` does not use up any of the allowance.
//!
//! A network that re-authenticates `api.github.com` for you (the development sandbox's egress proxy does) answers for its own
//! account, whatever token is sent: the numbers then describe that account, and the made-up-token test reports itself inconclusive.
use std::time::Duration;

use reclaw_net::{Net, NetConfig, Provider, Quota, Request, Source, Verdict};

fn net(dir: &tempfile::TempDir) -> Net {
    let (mut config, problems) = NetConfig::from_env(|k| std::env::var(k).ok());
    assert!(problems.is_empty(), "{problems:?}");
    config.cache_dir = Some(dir.path().to_path_buf());
    Net::new(config).expect("net")
}

fn quota(net: &Net) -> (bool, Quota) {
    match net.check_token(Provider::GitHub).expect("GitHub answers the rate-limit check") {
        Verdict::Accepted(info) => (info.had_token, info.quota.expect("the answer states the allowance")),
        Verdict::Rejected => panic!("GitHub refused the token in GITHUB_TOKEN"),
    }
}

#[test]
#[ignore = "uses the real GitHub API"]
fn github_states_the_allowance_and_a_not_modified_answer_is_free() {
    let dir = tempfile::tempdir().expect("tempdir");
    let net = net(&dir);

    let (had_token, before) = quota(&net);
    println!(
        "allowance: {} an hour ({}), {} left, resets at {}",
        before.limit,
        if had_token { "token" } else { "anonymous" },
        before.remaining,
        before.reset_at
    );
    assert_eq!(before.limit, if had_token { before.limit.max(1000) } else { 60 }, "60 an hour anonymous, thousands with a token");

    // Any public repository will do; `RECLAW_LIVE_REPO=owner/name` picks another (a restricted network may only allow some).
    let repo = std::env::var("RECLAW_LIVE_REPO").unwrap_or_else(|_| "tgeorgiadis/quiver-community-app-catalog".to_string());
    let request = Request::get(format!("https://api.github.com/repos/{repo}"))
        .accept("application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .cached(Duration::ZERO, false);
    let first = net.fetch(&request).unwrap_or_else(|e| panic!("{repo} from the network: {e} ({:?})", e.hint()));
    assert_eq!(first.source, Source::Network);
    assert!(first.etag.is_some(), "GitHub sends a validator");
    let second = net.fetch(&request).expect("the repository, asked again with its validator");
    assert_eq!(second.source, Source::CacheRevalidated, "answered 304");
    assert_eq!(first.body, second.body);

    let (_, after) = quota(&net);
    println!("left after one full answer and one 304: {}", after.remaining);
    assert_eq!(
        before.remaining - after.remaining,
        1,
        "the first answer counted, the 304 did not (another client on this address can disturb this)"
    );
}

#[test]
#[ignore = "uses the real GitHub API"]
fn a_made_up_token_is_refused_dropped_and_the_next_request_still_works() {
    use reclaw_log::Secret;
    use reclaw_net::TokenState;

    // Not from the environment: a token GitHub has never issued. It must be refused, and from then on not sent.
    let dir = tempfile::tempdir().expect("tempdir");
    let config = NetConfig {
        cache_dir: Some(dir.path().to_path_buf()),
        tokens: vec![("api.github.com".into(), Secret::new("ghp_ThisIsNotARealTokenAndNeverWas000000"))],
        ..NetConfig::default()
    };
    let net = Net::new(config).expect("net");

    let verdict = net.check_token(Provider::GitHub).expect("GitHub answers");
    println!("verdict for a made-up token: {verdict:?}; state afterwards: {:?}", net.token_state(Provider::GitHub));
    if let Verdict::Accepted(info) = &verdict {
        // GitHub cannot accept a token it never issued, so something between here and GitHub replaced the credential: a sandbox's
        // egress proxy does exactly this (and then `GITHUB_TOKEN` is a short placeholder). The test says so rather than failing.
        println!(
            "INCONCLUSIVE: the answer ({:?}) is for a different account than the one sent; this network re-authenticates api.github.com",
            info.quota
        );
        return;
    }
    assert_eq!(verdict, Verdict::Rejected, "GitHub answers 401 Bad credentials");
    assert_eq!(net.token_state(Provider::GitHub), TokenState::Rejected);

    // The same client carries on without the dead token and gets the anonymous allowance.
    let Verdict::Accepted(anonymous) = net.check_token(Provider::GitHub).expect("anonymous") else { panic!("accepted") };
    println!("then anonymous: {:?}", anonymous.quota);
    assert!(!anonymous.had_token);
}
