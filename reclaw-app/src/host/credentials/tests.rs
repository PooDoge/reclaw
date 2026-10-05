use std::{collections::VecDeque, path::PathBuf};

use reclaw_net::{NetError, Quota, TokenInfo, TokenState, testing::local_config};
use reclaw_ui::notices::NoticeKind;

use super::*;
use crate::host::{HostConfig, Initial, tests::Collector};

const GOOD: &str = "ghp_0123456789abcdefghijklmnopqrstuvwxyz";
const OTHER: &str = "ghp_zyxwvutsrqponmlkjihgfedcba9876543210";
const ENV: &str = "ghp_environment_token_000000000000000";

type Script = Arc<Mutex<VecDeque<Result<Verdict, NetError>>>>;

struct Rig {
    host: Host,
    /// What the host said at start, to be shown once the window is up.
    initial: Initial,
    sink: Arc<Collector>,
    net: Net,
    answers: Script,
    secrets: PathBuf,
    _dir: tempfile::TempDir,
}

fn accepted(limit: u64, remaining: u64) -> Result<Verdict, NetError> {
    Ok(Verdict::Accepted(TokenInfo { had_token: true, quota: Some(Quota { limit, remaining, reset_at: 0 }), ..TokenInfo::default() }))
}

fn rig_with(saved: Option<&str>, env: Vec<EnvToken>) -> Rig {
    let dir = tempfile::tempdir().expect("tempdir");
    let secrets = dir.path().join("config").join("secrets.toml");
    if let Some(text) = saved {
        std::fs::create_dir_all(secrets.parent().expect("dir")).expect("mkdir");
        std::fs::write(&secrets, text).expect("write");
    }
    let net = Net::new(local_config()).expect("net");
    let answers: Script = Arc::default();
    let script = answers.clone();
    let checker: Checker = Arc::new(move |_| {
        script.lock().expect("lock").pop_front().unwrap_or_else(|| Err(NetError::Other("no answer was scripted".into())))
    });
    let sink = Arc::new(Collector::default());
    let (host, initial) = Host::open(
        HostConfig {
            secrets_file: Some(secrets.clone()),
            env_tokens: env,
            checker: Some(checker),
            ..HostConfig::new(Some(net.clone()), dir.path().join("data").join("apps.json"))
        },
        sink.clone(),
    );
    Rig { host, initial, sink, net, answers, secrets, _dir: dir }
}

fn rig() -> Rig {
    rig_with(None, Vec::new())
}

fn env_github() -> Vec<EnvToken> {
    vec![EnvToken { provider: Provider::GitHub, variable: "GITHUB_TOKEN".into(), token: Secret::new(ENV) }]
}

impl Rig {
    fn answer(&self, answer: Result<Verdict, NetError>) {
        self.answers.lock().expect("lock").push_back(answer);
    }

    fn statuses(&self, provider: Provider) -> Vec<TokenStatus> {
        self.sink
            .all()
            .into_iter()
            .filter_map(|a| match a {
                AppAction::Credentials { provider: p, status } if p == provider => Some(status),
                _ => None,
            })
            .collect()
    }

    fn last_status(&self, provider: Provider) -> TokenStatus {
        self.statuses(provider).pop().expect("a status was sent")
    }

    fn titles(&self) -> Vec<String> {
        self.sink.notices().into_iter().map(|n| n.title).collect()
    }

    fn save(&self, text: &str) {
        self.host.save_token(Provider::GitHub, &Secret::new(text));
        // The check runs on its own thread; wait for the last status to stop being "checking".
        let end = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < end {
            if self.statuses(Provider::GitHub).last().is_none_or(|s| s.check != TokenCheck::Checking) && !self.sink.notices().is_empty() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}

#[test]
fn only_text_that_can_be_a_token_is_accepted_and_the_message_never_repeats_it() {
    assert_eq!(validate(&format!("  {GOOD}\n")).map(|s| s.expose().to_string()), Ok(GOOD.to_string()), "trimmed");
    for bad in ["", "   ", "two words", "with\nnewline", "tökén-with-accents", &"a".repeat(600)] {
        let why = validate(bad).expect_err(bad);
        assert!(!why.contains(bad) || bad.is_empty() || bad.trim().is_empty(), "{why}");
        assert!(why.len() > 20, "a sentence a person can act on: {why}");
    }
}

#[test]
fn the_environments_tokens_are_found_in_priority_order_and_blank_ones_are_ignored() {
    let vars =
        |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_string());
    let found = EnvToken::from_env(vars(&[("GITHUB_TOKEN", " a-plain "), ("RECLAW_GITHUB_TOKEN", "the-own-one"), ("GITLAB_TOKEN", "   ")]));
    assert_eq!(found.len(), 1);
    assert_eq!(
        (found[0].provider, found[0].variable.as_str(), found[0].token.expose()),
        (Provider::GitHub, "RECLAW_GITHUB_TOKEN", "the-own-one")
    );
}

#[test]
fn with_nothing_saved_and_nothing_in_the_environment_there_is_no_token() {
    let rig = rig();
    assert_eq!(rig.host.inner.tokens.status(), CredentialsStatus::default());
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::None);
}

#[test]
fn the_environments_token_is_used_and_labelled_with_its_variable_until_one_is_saved() {
    let rig = rig_with(None, env_github());
    let status = rig.host.inner.tokens.status();
    assert_eq!(status.github.source, TokenSource::Environment("GITHUB_TOKEN".into()));
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::Active);
    assert_eq!(status.github.short(Provider::GitHub), "From GITHUB_TOKEN");
}

#[test]
fn a_saved_token_wins_over_the_environments_and_is_read_from_the_file_at_start() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{GOOD}\"\n")), env_github());
    assert_eq!(rig.host.inner.tokens.status().github.source, TokenSource::Saved);
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::Active);
}

#[test]
fn a_pasted_token_that_the_service_accepts_is_saved_privately_used_and_reported_without_ever_appearing() {
    let rig = rig();
    rig.answer(accepted(5000, 4987));
    rig.save(&format!("  {GOOD}  "));

    let kinds: Vec<TokenCheck> = rig.statuses(Provider::GitHub).into_iter().map(|s| s.check).collect();
    assert_eq!(kinds[0], TokenCheck::Checking, "the screens see it being checked first");
    let last = rig.last_status(Provider::GitHub);
    assert_eq!(last.source, TokenSource::Saved);
    assert!(matches!(&last.check, TokenCheck::Accepted { quota: Some(q), had_token: true, .. } if q.remaining == 4987), "{last:?}");
    assert_eq!(last.short(Provider::GitHub), "4,987 of 5,000 left");
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::Active);

    let notices = rig.sink.notices();
    assert_eq!((notices[0].title.as_str(), notices[0].kind), ("GitHub token saved", NoticeKind::Note));
    assert!(notices[0].body.contains("4,987 of 5,000 requests left"), "{:?}", notices[0].body);

    let file = std::fs::read_to_string(&rig.secrets).expect("saved");
    assert!(file.contains(GOOD), "kept for the next start");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&rig.secrets).expect("meta").permissions().mode() & 0o777, 0o600);
    }
    // Nothing the screens were sent can contain the token.
    let everything = format!("{:?}", rig.sink.all());
    assert!(!everything.contains("0123456789abcdef"), "a token reached the screens: {everything}");
}

#[test]
fn a_token_the_service_refuses_is_not_kept() {
    let rig = rig();
    rig.answer(Ok(Verdict::Rejected));
    rig.save(GOOD);
    assert!(!rig.secrets.exists(), "nothing was written, or what was written was taken back");
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::None, "and it is no longer sent");
    assert_eq!(rig.last_status(Provider::GitHub), TokenStatus::default());
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("GitHub refused this token", NoticeKind::Problem));
    assert_eq!(notice.body, "It was not saved");
    assert!(!format!("{notice:?}").contains(GOOD));
}

#[test]
fn a_refused_paste_puts_the_environments_token_back_instead_of_leaving_none() {
    let rig = rig_with(None, env_github());
    rig.answer(Ok(Verdict::Rejected));
    rig.save(GOOD);
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::Active, "the environment's token is in use again");
    assert_eq!(rig.last_status(Provider::GitHub).source, TokenSource::Environment("GITHUB_TOKEN".into()));
}

#[test]
fn when_the_service_cannot_be_asked_the_token_is_kept_and_said_to_be_untested() {
    let rig = rig();
    rig.answer(Err(NetError::Timeout));
    rig.save(GOOD);
    assert!(std::fs::read_to_string(&rig.secrets).expect("kept").contains(GOOD));
    assert_eq!(rig.last_status(Provider::GitHub).check, TokenCheck::Failed);
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("GitHub token saved", NoticeKind::Note));
    assert!(notice.body.contains("untested"), "{:?}", notice.body);
}

#[test]
fn text_that_is_not_a_token_is_refused_before_anything_is_written() {
    let rig = rig();
    rig.host.save_token(Provider::GitHub, &Secret::new("two words"));
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("That is not a GitHub token", NoticeKind::Problem));
    assert!(!notice.body.contains("two words"));
    assert!(!rig.secrets.exists());
    assert!(rig.statuses(Provider::GitHub).is_empty(), "nothing changed on screen");
}

#[test]
fn a_tokens_file_that_cannot_be_read_is_never_written_over() {
    let rig = rig_with(Some("version = 9\n[tokens]\ngithub = \"future-format-token-0000\"\n"), Vec::new());
    let at_start: Vec<&str> = rig.initial.notices.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(at_start, ["Your saved tokens"], "told at start, with the window");
    let before = std::fs::read_to_string(&rig.secrets).expect("read");
    rig.host.save_token(Provider::GitHub, &Secret::new(GOOD));
    assert!(rig.titles().contains(&"Your saved tokens cannot be changed".to_string()), "{:?}", rig.titles());
    assert_eq!(std::fs::read_to_string(&rig.secrets).expect("read"), before);
}

#[test]
fn removing_the_saved_token_deletes_it_and_goes_back_to_the_environments() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{GOOD}\"\n")), env_github());
    rig.answer(accepted(15000, 14000));
    rig.host.remove_token(Provider::GitHub);
    assert!(!rig.secrets.exists(), "the file is removed with its last token");
    assert_eq!(rig.last_status(Provider::GitHub).source, TokenSource::Environment("GITHUB_TOKEN".into()));
    assert_eq!(rig.net.token_state(Provider::GitHub), TokenState::Active);
    let notice = rig.sink.notices().remove(0);
    assert_eq!(notice.title, "GitHub token removed");
    assert!(notice.details.iter().any(|d| d.contains("revoke")), "{:?}", notice.details);
}

#[test]
fn removing_when_nothing_is_saved_says_so() {
    let rig = rig();
    rig.host.remove_token(Provider::GitHub);
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("No saved GitHub token", NoticeKind::Note));
}

#[test]
fn a_token_found_to_be_dead_during_ordinary_work_is_reported_once_and_marked() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{OTHER}\"\n")), Vec::new());
    rig.host.token_refused(Provider::GitHub);
    assert_eq!(rig.last_status(Provider::GitHub).check, TokenCheck::Rejected);
    assert_eq!(rig.last_status(Provider::GitHub).short(Provider::GitHub), "GitHub refused it");
    let notice = rig.sink.notices().remove(0);
    assert_eq!((notice.title.as_str(), notice.kind), ("GitHub refused the token", NoticeKind::Problem));
    assert!(std::fs::read_to_string(&rig.secrets).expect("still saved").contains(OTHER), "kept, so the person can see what it was");
}

#[test]
fn a_refusal_found_by_a_running_check_is_left_to_the_check_to_report() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{OTHER}\"\n")), Vec::new());
    rig.host.tokens().checking.lock().expect("lock").insert("github");
    rig.host.token_refused(Provider::GitHub);
    assert_eq!(rig.last_status(Provider::GitHub).check, TokenCheck::Rejected);
    assert!(rig.sink.notices().is_empty(), "no second notice: {:?}", rig.titles());
}

#[test]
fn the_check_at_start_is_quiet_when_all_is_well_and_loud_when_a_saved_token_has_died() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{OTHER}\"\n")), Vec::new());
    rig.answer(accepted(5000, 4999));
    rig.host.check_now(Provider::GitHub, Reason::Quiet);
    assert!(rig.sink.notices().is_empty(), "{:?}", rig.titles());
    assert_eq!(rig.last_status(Provider::GitHub).short(Provider::GitHub), "4,999 of 5,000 left");

    rig.answer(Ok(Verdict::Rejected));
    rig.host.check_now(Provider::GitHub, Reason::Quiet);
    assert_eq!(rig.titles(), ["GitHub refused the token"]);
    assert_eq!(rig.last_status(Provider::GitHub).check, TokenCheck::Rejected);
    assert!(
        std::fs::read_to_string(&rig.secrets).expect("kept").contains(OTHER),
        "a dead saved token is not deleted behind the person's back"
    );

    rig.answer(Err(NetError::Timeout));
    rig.host.check_now(Provider::GitHub, Reason::Quiet);
    assert_eq!(rig.titles().len(), 1, "an unreachable service at start is not worth a message");
    assert_eq!(rig.last_status(Provider::GitHub).check, TokenCheck::Failed);
}

#[test]
fn asking_for_a_check_always_gets_an_answer_and_gitlab_without_a_token_has_nothing_to_check() {
    let rig = rig_with(Some(&format!("version = 1\n[tokens]\ngithub = \"{OTHER}\"\n")), Vec::new());
    rig.answer(Err(NetError::Timeout));
    rig.host.check_now(Provider::GitHub, Reason::Asked);
    assert_eq!(rig.sink.notices().remove(0).kind, NoticeKind::Problem, "an explicit question deserves a visible failure");
    rig.host.check_now(Provider::GitLab, Reason::Asked);
    assert!(rig.titles().contains(&"No GitLab token to check".to_string()), "{:?}", rig.titles());
}

#[test]
fn the_allowance_line_says_when_the_window_resets() {
    let q = |remaining, reset_at| Quota { limit: 5000, remaining, reset_at };
    assert_eq!(quota_line(&q(4987, 10_000 + 41 * 60 + 5), 10_000), "4,987 of 5,000 requests left, resets in 41 minutes");
    assert_eq!(quota_line(&q(10, 10_030), 10_000), "10 of 5,000 requests left, resets in under a minute");
    assert_eq!(quota_line(&q(10, 0), 10_000), "10 of 5,000 requests left", "no reset time known");
    assert_eq!(quota_line(&q(10, 9_000), 10_000), "10 of 5,000 requests left", "a reset already past says nothing");
}
