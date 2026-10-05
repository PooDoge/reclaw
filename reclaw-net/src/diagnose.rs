//! What actually happens when a host is contacted from this machine: not "is the internet up" but, for each host the program
//! needs, did the name resolve, did the connection and the certificate work, what did the server answer, and if it refused,
//! was it the network's policy, a bot check, a rate limit or an ordinary error. Run it on the machine that has the problem.
use std::time::{Duration, Instant};

use reqwest::header::{HeaderValue, RANGE};

use crate::{
    error::NetError,
    net::{Attempt, Net},
};

/// The hosts and addresses the launcher depends on, with what each is for.
pub const HOSTS: &[(&str, &str)] = &[
    ("https://raw.githubusercontent.com/tgeorgiadis/quiver-community-app-catalog/main/index.json", "community catalog index"),
    ("https://raw.githubusercontent.com/tgeorgiadis/quiver-community-app-catalog/main/platform-index.json", "platform metadata"),
    ("https://api.github.com/rate_limit", "GitHub API (release lists)"),
    ("https://github.com/HarbourMasters/Ghostship/releases/latest", "GitHub release downloads"),
    ("https://gitlab.com/api/v4/projects/sonicdcer%2FMarioKart64Recomp/releases?per_page=1", "GitLab API (release lists)"),
    ("https://thunderstore.io/api/experimental/community/", "Thunderstore (mods)"),
    ("https://gamebanana.com/apiv11/Util/Game/Index", "GameBanana (mods)"),
    ("https://cdn2.steamgriddb.com/icon_thumb/e260cb761dbaa59a1568e65c872a0951.png", "SteamGridDB artwork"),
];

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Reached {
    pub status: u16,
    pub http_version: String,
    pub server: Option<String>,
    pub content_type: Option<String>,
    /// Present on many sites; only a browser acts on it.
    pub content_security_policy: Option<String>,
    pub rate_limit_remaining: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Probe {
    pub url: String,
    pub elapsed: Duration,
    /// `Err(Status ..)` still means the host was reached.
    pub result: Result<Reached, NetError>,
}

impl Probe {
    /// One line for a table.
    pub fn summary(&self) -> String {
        match &self.result {
            Ok(r) => format!(
                "ok {} {} in {} ms{}",
                r.status,
                r.http_version,
                self.elapsed.as_millis(),
                r.server.as_ref().map(|s| format!(" (server: {s})")).unwrap_or_default()
            ),
            Err(e) => format!("{e}{}", e.hint().map(|h| format!(" -- {h}")).unwrap_or_default()),
        }
    }
}

impl Net {
    /// Contact `url` the way the program would, read at most a kilobyte, and report. Never retries: a diagnosis should show the
    /// first failure, not the third.
    pub fn probe(&self, url: &str) -> Probe {
        let started = Instant::now();
        let result = self.block(self.probe_async(url)).and_then(|r| r);
        Probe { url: url.to_string(), elapsed: started.elapsed(), result }
    }

    async fn probe_async(&self, text: &str) -> Result<Reached, NetError> {
        let url = self.parse(text)?;
        let host = url.host_str().unwrap_or_default().to_string();
        let mut builder =
            self.inner.api.get(url.clone()).timeout(Duration::from_secs(20)).header(RANGE, HeaderValue::from_static("bytes=0-1023"));
        if let Some(auth) = self.auth_header(&url) {
            builder = builder.header(reqwest::header::AUTHORIZATION, auth);
        }
        let response = builder.send().await.map_err(|e| match self.transport(&host, &e) {
            Attempt::Retry { error, .. } | Attempt::Stop(error) => error,
        })?;
        if !response.status().is_success() {
            return Err(match self.rejected(&host, response).await {
                Attempt::Retry { error, .. } | Attempt::Stop(error) => error,
            });
        }
        let text = |name: &str| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
        Ok(Reached {
            status: response.status().as_u16(),
            http_version: format!("{:?}", response.version()),
            server: text("server"),
            content_type: text("content-type"),
            content_security_policy: text("content-security-policy"),
            rate_limit_remaining: crate::limits::remaining(response.headers()),
        })
    }
}
