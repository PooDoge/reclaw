//! Which release of a quiverlauncher.com app to install, and whether the person has to confirm it first: a port of Quiver 3.5's
//! `CatalogReleases` and the checks in its install service. Pure; the job reads the site and acts on what this says.
//!
//! * The target is the person's pin, else the release the site verified, else the newest one it has not blocked.
//! * A download is checked against the SHA-256 the site recorded (a rolling release, rebuilt under one tag, against its host's own).
//! * An unverified release, or a verified one several antivirus engines flag, installs only when Install is pressed again; a blocked
//!   one only when it is pressed twice more. Quiver asks in a dialog; Reclaw has no dialog for this (ADR 0025), so the notice says what
//!   is wrong and the next press within [`WINDOW`] is the answer.
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use reclaw_catalog::site::{HistoryRelease, ReleaseState, Verdict};
use reclaw_install::{Asset, Release, version};

/// How long a press waits for the one that confirms it.
pub const WINDOW: Duration = Duration::from_secs(120);

/// Said when the site could not be read, so nothing is known about the release.
pub const UNREACHABLE: &str = "Reclaw could not reach quiverlauncher.com to check this release.";
/// Said when the site lists the app but not this release.
pub const NOT_SEEN: &str = "quiverlauncher.com has not seen this release yet.";

fn sha256_of(checksum: Option<&str>) -> Option<String> {
    let text = checksum?.trim();
    let hex = text.get(..7).filter(|p| p.eq_ignore_ascii_case("sha256:")).map(|_| &text[7..])?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hex.to_ascii_lowercase())
}

fn is_https(url: &str) -> bool {
    url.get(..8).is_some_and(|p| p.eq_ignore_ascii_case("https://")) && url.len() > 8
}

/// A release from the site in the shape the installer takes. Files without an https link are left out, as in Quiver; a rolling
/// release keeps no checksums (the file may have been rebuilt since the site read it), so the caller asks the host for it instead.
pub fn to_release(release: &HistoryRelease) -> Release {
    let assets = release
        .assets
        .iter()
        .filter_map(|a| {
            let url = a.url.as_deref().map(str::trim).filter(|u| is_https(u))?;
            let name = a.filename.trim();
            (!name.is_empty()).then(|| Asset {
                name: name.to_string(),
                url: url.to_string(),
                size: a.size.filter(|s| s.is_finite() && *s >= 1.).map(|s| s as u64),
                sha256: if release.rolling { None } else { sha256_of(a.checksum.as_deref()) },
            })
        })
        .collect();
    Release {
        tag: release.version.trim().to_string(),
        name: release.version.trim().to_string(),
        prerelease: release.prerelease,
        notes: release.notes.clone(),
        published: String::new(),
        page: release.upstream_url.clone().unwrap_or_default(),
        assets,
    }
}

fn installable(release: &HistoryRelease) -> bool {
    !to_release(release).assets.is_empty()
}

/// The release to install: the pin if the site lists it (one it does not is for the host to find: `None`), else the verified one
/// (pre-release or not), else the newest the site has not blocked, stable before pre-release unless pre-releases are wanted. Only
/// releases with files.
pub fn target<'a>(
    history: &'a [HistoryRelease],
    pin: Option<&str>,
    verified: Option<&str>,
    prerelease: bool,
) -> Option<&'a HistoryRelease> {
    if let Some(pin) = pin.map(str::trim).filter(|p| !p.is_empty()) {
        return history.iter().find(|r| version::equivalent(&r.version, pin) && installable(r));
    }
    // The feed's verified release, else the newest one verified with files. A verified pre-release counts: the site vouched for it,
    // and Quiver installs it as if it were pinned.
    let usable = |r: &&HistoryRelease| r.state == ReleaseState::Verified && installable(r);
    let verified = verified
        .and_then(|v| history.iter().find(|r| version::equivalent(&r.version, v)))
        .filter(usable)
        .or_else(|| history.iter().find(usable));
    if let Some(release) = verified {
        return Some(release);
    }
    let mut open = history.iter().filter(|r| r.state != ReleaseState::Blocked && installable(r));
    if prerelease {
        return open.next();
    }
    let open: Vec<_> = open.collect();
    open.iter().find(|r| !r.prerelease).or_else(|| open.first()).copied()
}

/// The newest verified release other than `excluding` that still lists files: what to offer when that one's files are gone. (Its
/// files are found again when it is installed: from the site, or from the repository when the site's links are not https.)
pub fn fallback<'a>(history: &'a [HistoryRelease], excluding: &str) -> Option<&'a HistoryRelease> {
    history.iter().find(|r| r.state == ReleaseState::Verified && !version::equivalent(&r.version, excluding) && !r.assets.is_empty())
}

/// What the site says about the one file about to be installed.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Check {
    Verified,
    Unverified(Vec<String>),
    Blocked(Vec<String>),
    /// Verified, but several antivirus engines flag the file: the person's own antivirus may block or remove it.
    Flagged {
        engines: Option<String>,
    },
}

impl Check {
    /// How many more presses of Install it takes.
    pub fn confirmations(&self) -> u8 {
        match self {
            Self::Verified => 0,
            Self::Unverified(_) | Self::Flagged { .. } => 1,
            Self::Blocked(_) => 2,
        }
    }

    /// Why it needs confirming, for the notice.
    pub fn reasons(&self) -> Vec<String> {
        match self {
            Self::Verified => Vec::new(),
            Self::Unverified(reasons) | Self::Blocked(reasons) => reasons.clone(),
            Self::Flagged { engines } => vec![match engines {
                Some(engines) => format!("VirusTotal: {engines} flag this file. Your antivirus may block or remove it."),
                None => "Several antivirus engines on VirusTotal flag this file. Your antivirus may block or remove it.".to_string(),
            }],
        }
    }
}

/// The check for `file` of the release the site lists as `release`. A file the site did not check, in a release whose other files it
/// did, is not verified either; VirusTotal's verdict is the file's own when the site gives one per file.
pub fn check(release: &HistoryRelease, file: &str) -> Check {
    let same = |name: &str| name.trim().eq_ignore_ascii_case(file.trim());
    let checked: Vec<_> = release.assets.iter().filter(|a| sha256_of(a.checksum.as_deref()).is_some()).collect();
    let mut reasons = release.reasons.clone();
    let unchecked_file = !checked.is_empty() && !checked.iter().any(|a| same(&a.filename));
    if unchecked_file {
        reasons.push(format!("quiverlauncher.com checked this release's other files, not {file}."));
    }
    match release.state {
        ReleaseState::Blocked => Check::Blocked(reasons),
        ReleaseState::Unverified => Check::Unverified(reasons),
        ReleaseState::Verified if unchecked_file => Check::Unverified(reasons),
        ReleaseState::Verified => {
            let per_file = release.assets.iter().any(|a| a.scan.is_some());
            let scan = if per_file {
                release.assets.iter().find(|a| same(&a.filename)).and_then(|a| a.scan.as_ref())
            } else {
                release.scan.as_ref()
            };
            match scan {
                Some(scan) if scan.verdict == Verdict::Flagged => Check::Flagged { engines: scan.engines.clone() },
                _ => Check::Verified,
            }
        }
    }
}

/// The check when the site did not list `version`: verified if it is the release the status feed calls verified, else not.
pub fn check_unlisted(version_: &str, verified: Option<&str>, read: bool) -> Check {
    if verified.is_some_and(|v| version::equivalent(v, version_)) {
        Check::Verified
    } else {
        Check::Unverified(vec![if read { NOT_SEEN } else { UNREACHABLE }.to_string()])
    }
}

struct Pending {
    version: String,
    presses: u8,
    at: Instant,
}

/// The presses of Install waiting to be confirmed, and the releases offered in place of one whose files were gone, by app.
#[derive(Default)]
pub struct Confirmations {
    pending: HashMap<u32, Pending>,
    offers: HashMap<u32, (String, Instant)>,
}

impl Confirmations {
    /// One press of Install that reached a release needing `needed` confirmations. True when this press is the last one needed (the
    /// record is then cleared); false when it must be pressed again. A press for anything else (another release, or the same tag with another
    /// file or checksum: `release` names all of them), or after [`WINDOW`], starts over.
    pub fn press(&mut self, app: u32, release: &str, needed: u8, now: Instant) -> bool {
        if needed == 0 {
            self.pending.remove(&app);
            return true;
        }
        let entry = self.pending.entry(app).or_insert(Pending { version: release.to_string(), presses: 0, at: now });
        if entry.version != release || now.duration_since(entry.at) > WINDOW {
            *entry = Pending { version: release.to_string(), presses: 0, at: now };
        }
        entry.presses += 1;
        entry.at = now;
        if entry.presses > needed {
            self.pending.remove(&app);
            return true;
        }
        false
    }

    /// How many more presses the app's pending release needs, after the one just recorded.
    pub fn remaining(&self, app: u32, needed: u8) -> u8 {
        self.pending.get(&app).map_or(needed, |p| (needed + 1).saturating_sub(p.presses))
    }

    /// Offer `release` to the next press of Install for `app`.
    pub fn offer(&mut self, app: u32, release: &str, now: Instant) {
        self.offers.insert(app, (release.to_string(), now));
    }

    /// The release offered to this press, if one was offered within [`WINDOW`]; taken once.
    pub fn take_offer(&mut self, app: u32, now: Instant) -> Option<String> {
        self.offers.remove(&app).filter(|(_, at)| now.duration_since(*at) <= WINDOW).map(|(release, _)| release)
    }
}

#[cfg(test)]
mod tests;
