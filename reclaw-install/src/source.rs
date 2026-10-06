//! Asking GitHub and GitLab which releases a project has. The answers are small and change rarely, so they are kept on disk
//! for a few minutes and checked with the server's validators after that (that is the network layer's job; the token
//! travels with the request the same way). Quiver's order of questions: for GitHub the single "latest" release first, the
//! whole list only when it is needed or the first answer will not do.
use std::time::Duration;

use reclaw_net::{Fetched, Net, NetError, Request};

use crate::{
    error::InstallError,
    release::{Release, parse_github_release, parse_gitlab_release, parse_list},
};

/// Where the two hosts' APIs are. Real addresses unless a test points them at a server of its own.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Api {
    pub github: String,
    pub gitlab: String,
}

impl Default for Api {
    fn default() -> Self {
        Self { github: "https://api.github.com".to_string(), gitlab: "https://gitlab.com/api/v4".to_string() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Host {
    GitHub,
    GitLab,
}

/// What a lookup found.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Releases {
    /// Newest first, as the host lists them.
    pub list: Vec<Release>,
    /// The tag the host itself calls the latest release (GitHub only).
    pub latest_tag: Option<String>,
    /// The answer was an old saved copy because the network failed.
    pub stale: bool,
}

#[derive(Clone)]
pub struct ReleaseSource {
    net: Net,
    api: Api,
}

/// How long an answer is used without asking again.
const FRESH_FOR: Duration = Duration::from_secs(5 * 60);
/// A release list is small; a hostile answer is not allowed to be large.
const MAX_ANSWER: u64 = 4 * 1024 * 1024;

/// `fresh`: the person asked (Install, Update, Check for updates), so a saved answer is only used if the server confirms it is
/// still current (a conditional request, which GitHub does not count against the rate limit when nothing changed). Without it a
/// saved answer under five minutes old is used with no question at all, which is right for a background look.
fn ttl(fresh: bool) -> Duration {
    if fresh { Duration::ZERO } else { FRESH_FOR }
}

fn github_request(url: String, fresh: bool) -> Request {
    Request::get(url)
        .accept("application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .max_bytes(MAX_ANSWER)
        .cached(ttl(fresh), true)
}

/// A path segment: `/` inside a GitLab project path must travel as `%2F`.
fn encode_project(path: &str) -> String {
    path.trim().bytes().fold(String::new(), |mut out, b| {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
        out
    })
}

/// `owner/name`, with nothing in it that could change which address is asked.
fn repo_is_plain(repo: &str) -> bool {
    !repo.is_empty()
        && !repo.starts_with('/')
        && !repo.ends_with('/')
        && !repo.contains("//")
        && repo
            .split('/')
            .all(|part| part != ".." && part != "." && part.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.')))
}

impl ReleaseSource {
    pub fn new(net: Net) -> Self {
        Self { net, api: Api::default() }
    }

    pub fn with_api(mut self, api: Api) -> Self {
        self.api = api;
        self
    }

    fn get(&self, request: &Request) -> Result<Fetched, NetError> {
        self.net.fetch(request)
    }

    /// The releases of `repo` (`owner/name`). With `whole_list` false, GitHub is asked for its latest release alone and the
    /// list only if that will not do (no such release, or no files in it).
    pub fn fetch(&self, host: Host, repo: &str, whole_list: bool, fresh: bool) -> Result<Releases, InstallError> {
        if !repo_is_plain(repo) {
            return Err(InstallError::BadAnswer(format!("\"{repo}\" is not a repository name")));
        }
        let found = match host {
            Host::GitHub => self.github(repo, whole_list, fresh)?,
            Host::GitLab => self.gitlab(repo, fresh)?,
        };
        if found.list.is_empty() {
            tracing::info!(repo, ?host, "the repository has no releases");
            return Err(InstallError::NoReleases { repo: repo.to_string() });
        }
        tracing::debug!(
            repo,
            ?host,
            releases = found.list.len(),
            latest = found.latest_tag.as_deref(),
            stale = found.stale,
            "releases found"
        );
        Ok(found)
    }

    fn github(&self, repo: &str, whole_list: bool, fresh: bool) -> Result<Releases, InstallError> {
        let base = self.api.github.trim_end_matches('/');
        let mut stale = false;
        let mut latest_tag = None;
        if !whole_list {
            match self.get(&github_request(format!("{base}/repos/{repo}/releases/latest"), fresh)) {
                Ok(answer) => {
                    stale = answer.is_stale();
                    let list = parse_list(&answer.body, parse_github_release).map_err(InstallError::BadAnswer)?;
                    latest_tag = list.first().map(|r| r.tag.clone());
                    if list.first().is_some_and(Release::has_assets) {
                        return Ok(Releases { list, latest_tag, stale });
                    }
                }
                // No "latest" release is a repository whose releases are all pre-releases, or none: the list tells which.
                Err(NetError::Status { status: 404, .. }) => {}
                Err(error) => return Err(error.into()),
            }
        }
        let answer = match self.get(&github_request(format!("{base}/repos/{repo}/releases?per_page=30"), fresh)) {
            Ok(answer) => answer,
            Err(NetError::Status { status: 404, .. }) => return Err(InstallError::NoReleases { repo: repo.to_string() }),
            Err(error) => return Err(error.into()),
        };
        let list = parse_list(&answer.body, parse_github_release).map_err(InstallError::BadAnswer)?;
        Ok(Releases { list, latest_tag, stale: stale || answer.is_stale() })
    }

    fn gitlab(&self, repo: &str, fresh: bool) -> Result<Releases, InstallError> {
        let base = self.api.gitlab.trim_end_matches('/');
        let request = Request::get(format!("{base}/projects/{}/releases", encode_project(repo)))
            .accept("application/json")
            .max_bytes(MAX_ANSWER)
            .cached(ttl(fresh), true);
        let answer = match self.get(&request) {
            Ok(answer) => answer,
            Err(NetError::Status { status: 404, .. }) => return Err(InstallError::NoReleases { repo: repo.to_string() }),
            Err(error) => return Err(error.into()),
        };
        let list = parse_list(&answer.body, parse_gitlab_release).map_err(InstallError::BadAnswer)?;
        Ok(Releases { list, latest_tag: None, stale: answer.is_stale() })
    }
}

#[cfg(test)]
mod tests;
