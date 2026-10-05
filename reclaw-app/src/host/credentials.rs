//! Access tokens: where they come from, saving a pasted one, asking the service whether it works, removing it, and what the
//! screens are told. The token itself lives in two places only, the tokens file and the network layer; the screens get a
//! [`TokenStatus`], which has no token in it.
//!
//! The rules, each for a reason:
//! * A token is validated for shape before anything else, and the message never repeats it.
//! * A pasted token is checked with the service. One the service refuses is **not kept**: a typo saved today is a mystery next week.
//! * A saved token wins over the environment's, because the person chose it here; the environment's is the fallback, and
//!   removing the saved one returns to it.
//! * A token the service refuses later (expired, revoked) stays saved but is no longer sent, and the person is told once.
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard, PoisonError, Weak},
    thread,
    time::{SystemTime, UNIX_EPOCH},
};

use reclaw_config::{Secrets, SecretsFile};
use reclaw_log::Secret;
use reclaw_net::{Net, NetError, Provider, Quota, Verdict};
use reclaw_ui::{
    credentials::{CredentialsStatus, TokenCheck, TokenSource, TokenStatus, group_digits},
    notices::Notice,
    store::AppAction,
};

use super::{Host, Inner};

/// Asks a service about its token. The network layer's `check_token` in the program; a test's own answers in a test.
pub type Checker = Arc<dyn Fn(Provider) -> Result<Verdict, NetError> + Send + Sync>;

/// A token the environment supplies, and the variable it came from (shown in Settings, so it is never a mystery where it came from).
#[derive(Clone)]
pub struct EnvToken {
    pub provider: Provider,
    pub variable: String,
    pub token: Secret,
}

impl EnvToken {
    /// The tokens named by `GITHUB_TOKEN`, `RECLAW_GITHUB_TOKEN` and the GitLab pair, for the variables that are set and not blank.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> Vec<Self> {
        Provider::ALL
            .into_iter()
            .filter_map(|provider| {
                provider.env_names().into_iter().find_map(|name| {
                    let value = get(name)?;
                    let value = value.trim();
                    (!value.is_empty()).then(|| Self { provider, variable: name.to_string(), token: Secret::new(value) })
                })
            })
            .collect()
    }
}

/// Why a check is being made, which decides what the person is told.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Reason {
    /// A token was just pasted: say whether it was accepted, and drop it if it was not.
    Saved,
    /// The person pressed Check token.
    Asked,
    /// At start: fill in the status line, and speak only if something is wrong.
    Quiet,
}

fn name(provider: Provider) -> &'static str {
    match provider {
        Provider::GitHub => "github",
        Provider::GitLab => "gitlab",
    }
}

struct Book {
    saved: Secrets,
    /// The tokens file could not be read, so it must not be written.
    writable: bool,
    status: CredentialsStatus,
}

/// The tokens the host knows about, and everything needed to change them.
pub(super) struct TokenBook {
    file: Option<SecretsFile>,
    env: Vec<EnvToken>,
    book: Mutex<Book>,
    /// Providers whose token is being checked right now. A refusal found by the check is reported by the check, not twice.
    checking: Mutex<HashSet<&'static str>>,
}

impl TokenBook {
    /// Read the saved tokens and give the network layer the ones to use. Notices to show once the window is up.
    pub(super) fn open(file: Option<PathBuf>, env: Vec<EnvToken>, net: Option<&Net>) -> (Self, Vec<Notice>) {
        let file = file.map(SecretsFile::at);
        let mut notices = Vec::new();
        let (saved, writable) = match &file {
            Some(file) => {
                let loaded = file.load();
                if let Some(warning) = loaded.warning {
                    notices.push(Notice::problem("Your saved tokens", &warning, vec![]));
                }
                (loaded.secrets, !loaded.read_only)
            }
            None => (Secrets::default(), true),
        };
        let this = Self {
            file,
            env,
            book: Mutex::new(Book { saved, writable, status: CredentialsStatus::default() }),
            checking: Mutex::default(),
        };
        for provider in Provider::ALL {
            let mut book = this.book();
            let status = this.status_for(&book.saved, provider, TokenCheck::Unchecked);
            book.status.set(provider, status);
            if let Some(net) = net {
                net.set_token(provider, this.token_in_use(&book.saved, provider));
            }
        }
        (this, notices)
    }

    fn book(&self) -> MutexGuard<'_, Book> {
        // The data behind the lock is plain and consistent between statements.
        self.book.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn status(&self) -> CredentialsStatus {
        self.book().status.clone()
    }

    fn env_for(&self, provider: Provider) -> Option<&EnvToken> {
        self.env.iter().find(|e| e.provider == provider)
    }

    /// The token requests should carry: the saved one, else the environment's.
    fn token_in_use(&self, saved: &Secrets, provider: Provider) -> Option<Secret> {
        saved.get(name(provider)).cloned().or_else(|| self.env_for(provider).map(|e| e.token.clone()))
    }

    fn status_for(&self, saved: &Secrets, provider: Provider, check: TokenCheck) -> TokenStatus {
        let source = if saved.get(name(provider)).is_some() {
            TokenSource::Saved
        } else if let Some(env) = self.env_for(provider) {
            TokenSource::Environment(env.variable.clone())
        } else {
            TokenSource::None
        };
        TokenStatus { source, check }
    }
}

/// Whether pasted text can be a token at all. The messages never repeat the text.
pub fn validate(text: &str) -> Result<Secret, &'static str> {
    let text = text.trim();
    if text.is_empty() {
        return Err("The token box is empty. Paste the token first, then choose Save token.");
    }
    if text.len() > 512 {
        return Err("That is too long to be a token. Paste only the token.");
    }
    if !text.chars().all(|c| c.is_ascii_graphic()) {
        return Err("A token is letters, digits and a few symbols, with no spaces or line breaks. Paste only the token.");
    }
    Ok(Secret::new(text))
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

/// "4,987 of 5,000 requests left, resets in 41 minutes".
pub(super) fn quota_line(quota: &Quota, now: u64) -> String {
    let left = format!("{} of {} requests left", group_digits(quota.remaining), group_digits(quota.limit));
    match quota.reset_at.checked_sub(now) {
        Some(secs) if quota.reset_at > 0 && secs >= 60 => format!("{left}, resets in {} minutes", secs / 60),
        Some(_) if quota.reset_at > 0 => format!("{left}, resets in under a minute"),
        _ => left,
    }
}

impl Host {
    fn tokens(&self) -> &TokenBook {
        &self.inner.tokens
    }

    fn send_status(&self, provider: Provider, status: TokenStatus) {
        self.send(AppAction::Credentials { provider, status });
    }

    /// Keep and use a pasted token, then ask the service about it.
    pub(super) fn save_token(&self, provider: Provider, pasted: &Secret) {
        let label = provider.label();
        let token = match validate(pasted.expose()) {
            Ok(token) => token,
            Err(why) => {
                tracing::info!(provider = label, "a pasted token was refused before saving: it is not shaped like a token");
                self.tell(Notice::problem(&format!("That is not a {label} token"), why, vec![]));
                return;
            }
        };
        let book = self.tokens();
        {
            let mut b = book.book();
            if !b.writable {
                drop(b);
                self.tell(Notice::problem(
                    "Your saved tokens cannot be changed",
                    "Their file could not be read, so Reclaw will not write over it",
                    vec!["Fix or move secrets.toml in Reclaw's config folder, then restart.".to_string()],
                ));
                return;
            }
            let previous = b.saved.get(name(provider)).cloned();
            b.saved.set(name(provider), Some(token.clone()));
            if let Some(file) = &book.file
                && let Err(error) = file.save(&b.saved)
            {
                b.saved.set(name(provider), previous);
                drop(b);
                tracing::error!(%error, "the token could not be saved");
                self.tell(Notice::problem(&format!("The {label} token could not be saved"), &error.to_string(), vec![]));
                return;
            }
            let status = book.status_for(&b.saved, provider, TokenCheck::Checking);
            b.status.set(provider, status.clone());
            drop(b);
            self.send_status(provider, status);
        }
        if let Some(net) = &self.inner.net {
            net.set_token(provider, Some(token));
        }
        tracing::info!(provider = label, kept_in_a_file = book.file.is_some(), "a token was saved");
        self.spawn_check(provider, Reason::Saved);
    }

    /// Stop using the saved token and forget it; go back to the environment's, if there is one.
    pub(super) fn remove_token(&self, provider: Provider) {
        let label = provider.label();
        let book = self.tokens();
        let mut b = book.book();
        if !b.writable {
            drop(b);
            self.tell(Notice::problem(
                "Your saved tokens cannot be changed",
                "Their file could not be read, so Reclaw will not write over it",
                vec![],
            ));
            return;
        }
        let had = b.saved.get(name(provider)).is_some();
        if !had {
            drop(b);
            self.tell(Notice::note(&format!("No saved {label} token"), "There is nothing to remove", vec![]));
            return;
        }
        let previous = b.saved.get(name(provider)).cloned();
        b.saved.set(name(provider), None);
        if let Some(file) = &book.file
            && let Err(error) = file.save(&b.saved)
        {
            b.saved.set(name(provider), previous);
            drop(b);
            tracing::error!(%error, "the token could not be removed");
            self.tell(Notice::problem(&format!("The {label} token could not be removed"), &error.to_string(), vec![]));
            return;
        }
        if let Some(old) = previous {
            reclaw_log::forget_secret(old.expose());
        }
        let in_use = book.token_in_use(&b.saved, provider);
        let status = book.status_for(&b.saved, provider, TokenCheck::Unchecked);
        let fallback = matches!(status.source, TokenSource::Environment(_));
        b.status.set(provider, status.clone());
        drop(b);
        if let Some(net) = &self.inner.net {
            net.set_token(provider, in_use);
        }
        self.send_status(provider, status);
        tracing::info!(provider = label, falls_back_to_environment = fallback, "the saved token was removed");
        self.tell(Notice::note(
            &format!("{label} token removed"),
            if fallback { "Reclaw is using the one from the environment again" } else { "Reclaw no longer uses it" },
            vec![format!("The token still exists on {}: revoke it there if you want it gone.", label.to_lowercase() + ".com")],
        ));
        self.spawn_check(provider, Reason::Quiet);
    }

    /// Ask in the background (the window must not wait for a service).
    pub(super) fn spawn_check(&self, provider: Provider, reason: Reason) {
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-token-check".into()).spawn(move || host.check_now(provider, reason));
        if let Err(error) = started {
            tracing::error!(%error, "the token check could not be started");
            self.tell(Notice::problem("The token could not be checked", "A background thread did not start", vec![error.to_string()]));
        }
    }

    /// Ask every service that has a token (and GitHub regardless, for the allowance Settings shows), without a word unless
    /// something is wrong. Called once at start.
    pub fn check_tokens(&self) {
        for provider in Provider::ALL {
            let has_token = self.tokens().status().of(provider).has_token();
            if has_token || provider == Provider::GitHub {
                self.spawn_check(provider, Reason::Quiet);
            }
        }
    }

    /// The question and what is done with the answer. Runs on a worker thread; also called directly by tests.
    pub(super) fn check_now(&self, provider: Provider, reason: Reason) {
        let label = provider.label();
        let Some(net) = &self.inner.net else {
            self.tell(Notice::problem("No network", "The network layer could not start, so tokens cannot be checked", vec![]));
            return;
        };
        let book = self.tokens();
        let has_token = book.status().of(provider).has_token();
        if !has_token && provider == Provider::GitLab {
            if reason == Reason::Asked {
                self.tell(Notice::note("No GitLab token to check", "Paste one first", vec![]));
            }
            return;
        }
        {
            let mut b = book.book();
            let status = TokenStatus { check: TokenCheck::Checking, ..b.status.of(provider).clone() };
            b.status.set(provider, status.clone());
            drop(b);
            self.send_status(provider, status);
        }
        book.checking.lock().unwrap_or_else(PoisonError::into_inner).insert(name(provider));
        let outcome = match &self.inner.checker {
            Some(checker) => checker(provider),
            None => net.check_token(provider),
        };
        book.checking.lock().unwrap_or_else(PoisonError::into_inner).remove(name(provider));

        let mut b = book.book();
        let source = book.status_for(&b.saved, provider, TokenCheck::Unchecked).source;
        match outcome {
            Ok(Verdict::Accepted(info)) => {
                let extra = info.scopes.clone().unwrap_or_default();
                let status = TokenStatus {
                    source,
                    check: TokenCheck::Accepted {
                        had_token: info.had_token,
                        quota: info.quota,
                        expires: info.expires.clone(),
                        extra_permissions: extra.clone(),
                    },
                };
                b.status.set(provider, status.clone());
                drop(b);
                self.send_status(provider, status);
                let allowance = info.quota.map(|q| quota_line(&q, now())).unwrap_or_else(|| "Accepted".to_string());
                let mut details = Vec::new();
                if let Some(expires) = &info.expires {
                    details.push(format!("It stops working on {expires}. Make a new one before then."));
                }
                if !extra.is_empty() {
                    details.push(format!(
                        "This token has permissions Reclaw does not need ({}). A token with none is safer: make a new one and leave every box unticked.",
                        extra.join(", ")
                    ));
                }
                match reason {
                    Reason::Saved => self.tell(Notice::note(&format!("{label} token saved"), &allowance, details)),
                    Reason::Asked => self.tell(Notice::note(
                        &format!("{label}: {}", if info.had_token { "token accepted" } else { "no token in use" }),
                        &allowance,
                        details,
                    )),
                    Reason::Quiet => {
                        tracing::info!(provider = label, %allowance, "token checked");
                        if !extra.is_empty() {
                            self.tell(Notice::note(
                                &format!("Your {label} token has more permissions than Reclaw needs"),
                                "A token with none is safer",
                                details,
                            ));
                        }
                    }
                }
            }
            Ok(Verdict::Rejected) => {
                if reason == Reason::Saved {
                    // A token the service refuses is not kept: forget it again, in the file and in the network layer.
                    let previous = b.saved.get(name(provider)).cloned();
                    b.saved.set(name(provider), None);
                    let saved_ok = book.file.as_ref().is_none_or(|file| file.save(&b.saved).is_ok());
                    if let Some(old) = previous {
                        reclaw_log::forget_secret(old.expose());
                    }
                    let in_use = book.token_in_use(&b.saved, provider);
                    let status = book.status_for(&b.saved, provider, TokenCheck::Unchecked);
                    b.status.set(provider, status.clone());
                    drop(b);
                    net.set_token(provider, in_use);
                    self.send_status(provider, status);
                    tracing::warn!(provider = label, removed_from_file = saved_ok, "the service refused the pasted token; it was not kept");
                    self.tell(Notice::problem(
                        &format!("{label} refused this token"),
                        "It was not saved",
                        vec![
                            "Check that the whole token was copied, and that it has not expired or been revoked.".to_string(),
                            "A GitHub token needs no permissions: leave every box unticked when you make it.".to_string(),
                        ],
                    ));
                } else {
                    let status = TokenStatus { source, check: TokenCheck::Rejected };
                    b.status.set(provider, status.clone());
                    drop(b);
                    self.send_status(provider, status);
                    self.refused_notice(provider);
                }
            }
            Err(error) => {
                let status = TokenStatus { source, check: TokenCheck::Failed };
                b.status.set(provider, status.clone());
                drop(b);
                self.send_status(provider, status);
                tracing::warn!(provider = label, %error, hint = error.hint(), "the token could not be checked");
                let hint = error.hint().map(str::to_string);
                let details: Vec<String> = std::iter::once(error.to_string()).chain(hint).collect();
                match reason {
                    Reason::Saved => self.tell(Notice::note(
                        &format!("{label} token saved"),
                        "It could not be checked just now, so it is untested",
                        details,
                    )),
                    Reason::Asked => {
                        self.tell(Notice::problem(&format!("{label} could not be asked about the token"), &error.to_string(), details))
                    }
                    Reason::Quiet => {}
                }
            }
        }
    }

    fn refused_notice(&self, provider: Provider) {
        let label = provider.label();
        self.tell(Notice::problem(
            &format!("{label} refused the token"),
            "It is not used any more; requests go out without it",
            vec![format!(
                "It may have expired or been revoked. Make a new one on {}, paste it in Settings (Network), or remove this one.",
                label.to_lowercase() + ".com"
            )],
        ));
    }

    /// When the service refuses a token during ordinary work, say so once and update the status line.
    pub(super) fn watch_for_refused_tokens(&self) {
        let Some(net) = self.inner.net.clone() else { return };
        // A weak reference: the network layer must not keep the host alive.
        let weak: Weak<Inner> = Arc::downgrade(&self.inner);
        net.on_token_rejected(move |host| {
            let (Some(inner), Some(provider)) = (weak.upgrade(), Provider::from_host(host)) else { return };
            Host { inner }.token_refused(provider);
        });
    }

    /// A service refused the token it was sent while the program was doing something else. Update the status line and, unless a
    /// check is running (which reports its own findings), tell the person once.
    pub(super) fn token_refused(&self, provider: Provider) {
        let checking = self.tokens().checking.lock().unwrap_or_else(PoisonError::into_inner).contains(name(provider));
        let mut b = self.tokens().book();
        let source = self.tokens().status_for(&b.saved, provider, TokenCheck::Unchecked).source;
        let status = TokenStatus { source, check: TokenCheck::Rejected };
        b.status.set(provider, status.clone());
        drop(b);
        self.send_status(provider, status);
        if !checking {
            self.refused_notice(provider);
        }
    }
}

#[cfg(test)]
mod tests;
