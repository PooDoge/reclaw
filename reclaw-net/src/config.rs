//! How the network layer behaves, and where each setting comes from. Everything has a default that works on a plain
//! connection; the environment adjusts it for the situations that need it (a proxy, a company certificate, a GitHub token).
use std::{fmt, path::PathBuf, time::Duration};

use sha2::{Digest, Sha256};

use crate::address::AddressPolicy;

/// Where this program says it comes from. GitHub requires a user agent and asks for one that identifies the client.
pub const PROJECT_URL: &str = "https://github.com/poodoge/reclaw";

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum ProxyMode {
    /// What the system says: `HTTPS_PROXY`, `ALL_PROXY` and `NO_PROXY`, and on Windows and macOS the system settings.
    #[default]
    System,
    /// Never use a proxy.
    None,
    /// Always this one (`http://host:port`).
    Url(String),
}

#[derive(Clone)]
pub struct NetConfig {
    pub user_agent: String,
    pub connect_timeout: Duration,
    /// A transfer that receives nothing for this long has stalled.
    pub stall_timeout: Duration,
    pub proxy: ProxyMode,
    /// Certificate authorities to trust besides the system's and the bundled ones, as PEM.
    pub extra_roots_pem: Option<Vec<u8>>,
    pub address_policy: AddressPolicy,
    pub max_redirects: usize,
    /// Requests in flight to one host at a time.
    pub per_host_concurrency: usize,
    /// Tries for one request, the first included.
    pub attempts: u32,
    /// Where cached answers are kept. Without it nothing is cached.
    pub cache_dir: Option<PathBuf>,
    /// `(host, token)`: sent as a bearer token to exactly that host and never anywhere a redirect leads.
    pub tokens: Vec<(String, String)>,
    /// Threads of the runtime that carries the requests.
    pub worker_threads: usize,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            user_agent: format!("Reclaw/{} (+{PROJECT_URL})", env!("CARGO_PKG_VERSION")),
            connect_timeout: Duration::from_secs(10),
            stall_timeout: Duration::from_secs(20),
            proxy: ProxyMode::System,
            extra_roots_pem: None,
            address_policy: AddressPolicy::Public,
            max_redirects: 6,
            per_host_concurrency: 6,
            attempts: 3,
            cache_dir: None,
            tokens: Vec::new(),
            worker_threads: 2,
        }
    }
}

impl fmt::Debug for NetConfig {
    /// Without the tokens: a configuration ends up in logs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NetConfig")
            .field("user_agent", &self.user_agent)
            .field("proxy", &self.proxy)
            .field("extra_roots", &self.extra_roots_pem.as_ref().map(Vec::len))
            .field("address_policy", &self.address_policy)
            .field("per_host_concurrency", &self.per_host_concurrency)
            .field("attempts", &self.attempts)
            .field("cache_dir", &self.cache_dir)
            .field("tokens_for", &self.tokens.iter().map(|(h, _)| h.as_str()).collect::<Vec<_>>())
            .finish_non_exhaustive()
    }
}

impl NetConfig {
    /// Defaults adjusted by the environment (`get` reads a variable). The second part lists what was asked for and could
    /// not be done, to show the user once.
    ///
    /// * `SSL_CERT_FILE`: a PEM bundle of extra certificate authorities.
    /// * `RECLAW_PROXY`: `none`, or a proxy address; otherwise the system's proxy settings apply.
    /// * `GITHUB_TOKEN` / `RECLAW_GITHUB_TOKEN`, `GITLAB_TOKEN` / `RECLAW_GITLAB_TOKEN`: raise the services' rate limits.
    pub fn from_env(get: impl Fn(&str) -> Option<String>) -> (Self, Vec<String>) {
        let mut config = Self::default();
        let mut problems = Vec::new();
        let var = |names: &[&str]| names.iter().find_map(|n| get(n).map(|v| v.trim().to_string()).filter(|v| !v.is_empty()));
        if let Some(path) = var(&["SSL_CERT_FILE"]) {
            match std::fs::read(&path) {
                Ok(pem) => config.extra_roots_pem = Some(pem),
                Err(e) => problems.push(format!("SSL_CERT_FILE {path} could not be read, so it is not used: {e}")),
            }
        }
        if let Some(proxy) = var(&["RECLAW_PROXY"]) {
            config.proxy = if proxy.eq_ignore_ascii_case("none") { ProxyMode::None } else { ProxyMode::Url(proxy) };
        }
        for (names, host) in
            [(&["RECLAW_GITHUB_TOKEN", "GITHUB_TOKEN"][..], "api.github.com"), (&["RECLAW_GITLAB_TOKEN", "GITLAB_TOKEN"][..], "gitlab.com")]
        {
            if let Some(token) = var(names) {
                config.tokens.push((host.to_string(), token));
            }
        }
        (config, problems)
    }

    /// The token for exactly this host, if there is one.
    pub fn token_for(&self, host: &str) -> Option<&str> {
        self.tokens.iter().find(|(h, _)| h.eq_ignore_ascii_case(host)).map(|(_, t)| t.as_str())
    }

    /// What separates one identity's cached answers from another's: a private repository's answer must not be served to a
    /// request without the token. `anon`, or a short fingerprint of the token (never the token).
    pub fn credential_tag(&self, host: &str) -> String {
        match self.token_for(host) {
            None => "anon".to_string(),
            Some(token) => {
                let digest = Sha256::digest(token.as_bytes());
                digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
            }
        }
    }
}

#[cfg(test)]
mod tests;
