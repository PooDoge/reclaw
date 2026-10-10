use std::time::Instant;

use reclaw_catalog::site::HistoryRelease;
use reclaw_install::{InstallError, Request, Resolved, version};
use reclaw_net::NetError;
use reclaw_sync::{DownloadProblem, SiteClient};

use super::verified::{self, Check};
use crate::host::{Host, community::Linked};

/// A job for an app the site lists: where to ask, and what it said.
pub(super) struct SiteJob {
    pub client: SiteClient,
    pub linked: Linked,
    /// The release history, once read; `None` when it could not be.
    pub history: Option<Vec<HistoryRelease>>,
}

impl SiteJob {
    /// Read the app's releases and point `request` at the one to install, with the checksums the site recorded. A site that cannot be
    /// read leaves the request to the host (the release is then confirmed as unverified, as Quiver does).
    pub fn plan(&mut self, request: &mut Request) {
        match self.client.release_history(&self.linked.slug) {
            Ok(history) => {
                let pin = request.preferred_version.clone();
                if let Some(target) = verified::target(&history, pin.as_deref(), self.linked.verified.as_deref(), request.allow_prerelease)
                {
                    tracing::info!(slug = %self.linked.slug, release = %target.version, state = ?target.state, rolling = target.rolling, "quiverlauncher.com chose the release");
                    request.preferred_version = Some(target.version.trim().to_string());
                    // A rolling release is rebuilt under one tag: its host's own digest is the one that matches today's file.
                    if !target.rolling {
                        request.releases = Some(vec![verified::to_release(target)]);
                    }
                }
                self.history = Some(history);
            }
            Err(error) => {
                tracing::warn!(slug = %self.linked.slug, %error, "quiverlauncher.com's releases could not be read; asking the repository")
            }
        }
    }

    /// A rolling release came from its repository; when the host states no digest for the file (GitLab, older GitHub uploads), the
    /// site's is checked instead, as Quiver accepts either.
    pub fn fill_digest(&self, resolved: &mut Resolved) {
        if resolved.asset.sha256.is_some() {
            return;
        }
        let tag = resolved.release.tag.as_str();
        let Some(release) = self.history.as_deref().and_then(|h| h.iter().find(|r| version::equivalent(&r.version, tag))) else { return };
        let mut listed = release.clone();
        listed.rolling = false;
        let name = resolved.asset.name.trim();
        if let Some(sum) =
            verified::to_release(&listed).assets.into_iter().find(|a| a.name.eq_ignore_ascii_case(name)).and_then(|a| a.sha256)
        {
            tracing::info!(release = tag, file = name, "the host states no digest; checking against quiverlauncher.com's");
            resolved.asset.sha256 = Some(sum);
        }
    }

    /// What the site says about the file about to be installed.
    pub fn check(&self, resolved: &Resolved) -> Check {
        let tag = resolved.release.tag.as_str();
        match self.history.as_deref().and_then(|h| h.iter().find(|r| version::equivalent(&r.version, tag))) {
            Some(release) => verified::check(release, &resolved.asset.name),
            None => verified::check_unlisted(tag, self.linked.verified.as_deref(), self.history.is_some()),
        }
    }

    /// A download that failed because its file is gone or is not the one the site checked is reported, so the site reads the release
    /// back now; for a missing file, the newest other verified release is offered. Returns what the person is told besides the error.
    pub fn after_failure(&self, error: &InstallError, resolved: &Resolved) -> Option<(DownloadProblem, Option<String>)> {
        let problem = match error {
            InstallError::Net(NetError::Integrity(_)) if resolved.asset.sha256.is_some() => DownloadProblem::Mismatch,
            InstallError::Net(NetError::Status { status: 404 | 410, .. }) => DownloadProblem::Missing,
            _ => return None,
        };
        let (slug, tag, file) = (&self.linked.slug, resolved.release.tag.as_str(), resolved.asset.name.as_str());
        match self.client.report_download_problem(slug, tag, file, problem) {
            Ok(()) => tracing::info!(slug = %slug, release = tag, file, ?problem, "told quiverlauncher.com about the broken download"),
            Err(error) => {
                tracing::warn!(slug = %slug, release = tag, file, ?problem, %error, "quiverlauncher.com could not be told about the broken download")
            }
        }
        let other = (problem == DownloadProblem::Missing)
            .then(|| self.history.as_deref().and_then(|h| verified::fallback(h, tag)).map(|r| r.version.trim().to_string()))
            .flatten();
        Some((problem, other))
    }
}

impl Host {
    /// The site's side of a job for `app`, when it is linked and the site can be asked.
    pub(super) fn site_job(&self, app: u32) -> Option<SiteJob> {
        let community = &self.inner.community;
        Some(SiteJob { client: community.client()?, linked: community.linked(app)?, history: None })
    }

    /// One press of Install reached a release that needs confirming: true when it is confirmed now.
    pub(super) fn confirmed(&self, app: u32, release: &str, check: &Check) -> (bool, u8) {
        let mut confirmations = self.inner.installs.confirmations();
        let needed = check.confirmations();
        let done = confirmations.press(app, release, needed, Instant::now());
        (done, if done { 0 } else { confirmations.remaining(app, needed) })
    }

    /// Offer `release` to the next press of Install.
    pub(super) fn offer_release(&self, app: u32, release: &str) {
        self.inner.installs.confirmations().offer(app, release, Instant::now());
    }

    /// The release offered to this press of Install, if any.
    pub(super) fn take_offer(&self, app: u32) -> Option<String> {
        self.inner.installs.confirmations().take_offer(app, Instant::now())
    }
}
