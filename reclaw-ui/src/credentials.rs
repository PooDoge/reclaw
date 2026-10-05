//! What the screens know about the access tokens: whether there is one and where it came from, whether the service accepted it, and
//! how many requests are left. Never the token itself: a token lives in the network layer and the tokens file, and nothing the UI
//! holds, shows or logs can contain one.
use reclaw_net::{Provider, Quota};

/// Where the token in use came from.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum TokenSource {
    #[default]
    None,
    /// Pasted in Settings and saved in the tokens file.
    Saved,
    /// An environment variable, by name (`GITHUB_TOKEN`). Used only when nothing is saved.
    Environment(String),
}

/// What the last question to the service came to.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum TokenCheck {
    #[default]
    Unchecked,
    Checking,
    Accepted {
        /// Whether the question was asked with a token (without one, the answer describes the anonymous allowance).
        had_token: bool,
        quota: Option<Quota>,
        /// When the token stops working, as the service writes it.
        expires: Option<String>,
        /// Permissions a GitHub classic token has beyond what Reclaw needs (none).
        extra_permissions: Vec<String>,
    },
    /// The service refused the token. It is no longer sent.
    Rejected,
    /// The service could not be asked (offline, blocked, limited).
    Failed,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct TokenStatus {
    pub source: TokenSource,
    pub check: TokenCheck,
}

impl TokenStatus {
    pub fn has_token(&self) -> bool {
        self.source != TokenSource::None
    }

    /// The short value on the Status row: one line, no time in it (it is shown again on every redraw).
    pub fn short(&self, provider: Provider) -> String {
        let anonymous_default = match provider {
            Provider::GitHub => "No token (60 requests an hour)".to_string(),
            Provider::GitLab => "No token".to_string(),
        };
        match (&self.source, &self.check) {
            (_, TokenCheck::Checking) => "Checking...".to_string(),
            (TokenSource::None, TokenCheck::Accepted { quota: Some(q), .. }) => {
                format!("No token ({} of {} left)", group_digits(q.remaining), group_digits(q.limit))
            }
            (TokenSource::None, _) => anonymous_default,
            (_, TokenCheck::Rejected) => format!("{} refused it", provider.label()),
            (_, TokenCheck::Failed) => "Could not check".to_string(),
            (_, TokenCheck::Accepted { quota: Some(q), extra_permissions, .. }) => {
                let base = format!("{} of {} left", group_digits(q.remaining), group_digits(q.limit));
                if extra_permissions.is_empty() { base } else { format!("{base}, has extra permissions") }
            }
            (_, TokenCheck::Accepted { quota: None, .. }) => "Accepted".to_string(),
            (TokenSource::Saved, TokenCheck::Unchecked) => "Saved".to_string(),
            (TokenSource::Environment(name), TokenCheck::Unchecked) => format!("From {name}"),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct CredentialsStatus {
    pub github: TokenStatus,
    pub gitlab: TokenStatus,
}

impl CredentialsStatus {
    pub fn of(&self, provider: Provider) -> &TokenStatus {
        match provider {
            Provider::GitHub => &self.github,
            Provider::GitLab => &self.gitlab,
        }
    }

    pub fn set(&mut self, provider: Provider, status: TokenStatus) {
        match provider {
            Provider::GitHub => self.github = status,
            Provider::GitLab => self.gitlab = status,
        }
    }
}

/// 4987 as "4,987".
pub fn group_digits(n: u64) -> String {
    let digits = n.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests;
