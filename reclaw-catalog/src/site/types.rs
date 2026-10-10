//! The answers of the quiverlauncher.com catalog API other than an app itself (`app.rs`), typed after the site's public responses (Quiver's `@quiverlauncher/api`
//! package and its C# `QuiverCatalogClient`). Read leniently: a field that is missing takes its default, a field the site added later
//! is ignored, and a word Reclaw does not know (a new release state, a new verdict) becomes the most careful one it does.
use serde::{Deserialize, Deserializer};

use super::SiteApp;

/// A count the site sends as a JavaScript number: anything that is not a finite positive number is zero.
pub(super) fn count<'de, D: Deserializer<'de>>(d: D) -> Result<u32, D::Error> {
    let n = Option::<f64>::deserialize(d)?.unwrap_or(0.);
    Ok(if n.is_finite() && n > 0. { n.min(f64::from(u32::MAX)) as u32 } else { 0 })
}

/// A time in JavaScript milliseconds that may be `null` (zero then).
pub(super) fn millis<'de, D: Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    Ok(Option::<f64>::deserialize(d)?.filter(|n| n.is_finite()).unwrap_or(0.))
}

/// A string that may be `null`.
pub(super) fn text<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

/// A list that may be `null`.
pub(super) fn list<'de, D: Deserializer<'de>, T: Deserialize<'de>>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
}

/// A list that may be `null`, or some other value standing for "none" (`false`). An item that does not read is left out.
fn list_or_none<'de, D: Deserializer<'de>, T: serde::de::DeserializeOwned>(d: D) -> Result<Vec<T>, D::Error> {
    Ok(match serde_json::Value::deserialize(d)? {
        serde_json::Value::Array(items) => items.into_iter().filter_map(|item| serde_json::from_value(item).ok()).collect(),
        _ => Vec::new(),
    })
}

/// How much of an app AI wrote, as the site judged it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AiLevel {
    /// None found, or the site has not said.
    #[default]
    None,
    Assisted,
    Generated,
}

impl From<String> for AiLevel {
    fn from(word: String) -> Self {
        match word.as_str() {
            "assisted" => Self::Assisted,
            "generated" => Self::Generated,
            _ => Self::None,
        }
    }
}

impl<'de> Deserialize<'de> for AiLevel {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Option::<String>::deserialize(d)?.map(Self::from).unwrap_or_default())
    }
}

/// What a player said about how an app ran.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunResult {
    Runs,
    Issues,
    Broken,
    /// A word this version does not know; shown as nothing in particular.
    Other,
}

impl<'de> Deserialize<'de> for RunResult {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(d)?.as_deref() {
            Some("runs") => Self::Runs,
            Some("issues") => Self::Issues,
            Some("broken") => Self::Broken,
            _ => Self::Other,
        })
    }
}

/// What the site says about one release. Only a verified one is ever an update; an unknown word counts as unverified, as in Quiver.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReleaseState {
    Verified,
    #[default]
    Unverified,
    /// Withdrawn, taken down, held, a file replaced or flagged: installed only when a player insists.
    Blocked,
}

impl<'de> Deserialize<'de> for ReleaseState {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(d)?.as_deref() {
            Some("verified") => Self::Verified,
            Some("blocked") => Self::Blocked,
            _ => Self::Unverified,
        })
    }
}

/// VirusTotal's verdict on a file or a release (from its most worrying file).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Verdict {
    Clean,
    /// One or two engines; often a false alarm.
    Warning,
    /// Several engines: the player's own antivirus may block or remove the files.
    Flagged,
    #[default]
    Pending,
    Missing,
}

impl<'de> Deserialize<'de> for Verdict {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(match Option::<String>::deserialize(d)?.as_deref() {
            Some("clean") => Self::Clean,
            Some("warning") => Self::Warning,
            Some("flagged") => Self::Flagged,
            Some("missing") => Self::Missing,
            _ => Self::Pending,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Scan {
    pub verdict: Verdict,
    /// "3 of 70 engines", for a warning or a flag.
    pub engines: Option<String>,
    /// The VirusTotal report of the file the verdict is from.
    pub url: Option<String>,
}

#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Verified {
    #[serde(deserialize_with = "text")]
    pub version: String,
    pub released_at: Option<f64>,
    pub prerelease: bool,
    /// Every file has a SHA-256 the site pinned.
    pub pinned: bool,
    /// Rebuilt under the same tag (a nightly).
    pub rolling: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Evidence {
    #[serde(deserialize_with = "text")]
    pub kind: String,
    #[serde(deserialize_with = "text")]
    pub detail: String,
    pub url: Option<String>,
}

/// How much AI wrote an app, and how the site knows.
#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AiUse {
    pub level: AiLevel,
    /// `developer`, `readme`, `signals` or `admin`.
    #[serde(deserialize_with = "text")]
    pub source: String,
    #[serde(deserialize_with = "list")]
    pub evidence: Vec<Evidence>,
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SiteProject {
    #[serde(deserialize_with = "text")]
    pub name: String,
    #[serde(deserialize_with = "text")]
    pub description: String,
    /// `github`, `gitlab` or `manual`.
    #[serde(deserialize_with = "text")]
    pub provider: String,
    pub repository: Option<String>,
    pub website: Option<String>,
    pub author: Option<String>,
    pub ai_use: Option<AiUse>,
}

/// A release the site took back, and why.
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Withdrawn {
    #[serde(deserialize_with = "text")]
    pub version: String,
    #[serde(deserialize_with = "text")]
    pub reason: String,
    #[serde(deserialize_with = "millis")]
    pub at: f64,
}

/// A newer release the site is checking before it is offered.
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Checking {
    #[serde(deserialize_with = "text")]
    pub version: String,
    /// When the wait ends; none while a maintainer has to look at it.
    pub check_ends_at: Option<f64>,
    pub needs_review: bool,
    /// Why it waits, worded for players.
    #[serde(deserialize_with = "list")]
    pub reasons: Vec<String>,
}

/// An app's page (`/apps/<slug>`).
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Detail {
    pub entry: SiteApp,
    pub project: SiteProject,
    /// The releases the site took back. The live API answers a plain `false` here when there are none (seen in Quiver 3.5's test
    /// of a real answer), so anything but a list is none.
    #[serde(deserialize_with = "list_or_none")]
    pub withdrawn: Vec<Withdrawn>,
    pub checking: Option<Checking>,
}

/// What one player said about how an app ran. Written on the website; read here.
#[derive(Clone, PartialEq, Debug, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Review {
    #[serde(deserialize_with = "text")]
    pub id: String,
    #[serde(deserialize_with = "text")]
    pub author: String,
    pub result: RunResult,
    #[serde(deserialize_with = "text")]
    pub body: String,
    pub platform: Option<String>,
    /// The release they tested, as tagged.
    pub version: Option<String>,
    #[serde(deserialize_with = "millis")]
    pub created_at: f64,
}

impl Default for Review {
    fn default() -> Self {
        Self {
            id: String::new(),
            author: String::new(),
            result: RunResult::Other,
            body: String::new(),
            platform: None,
            version: None,
            created_at: 0.,
        }
    }
}

#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Asset {
    #[serde(deserialize_with = "text")]
    pub filename: String,
    pub url: Option<String>,
    /// `sha256:<hex>` of the file the site saw when the release came out.
    pub checksum: Option<String>,
    pub size: Option<f64>,
    pub scan: Option<Scan>,
}

/// One release as the site judges it (`/apps/<slug>/release-history`): newest first, each verified, unverified or blocked, with why.
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryRelease {
    #[serde(deserialize_with = "text")]
    pub version: String,
    pub state: ReleaseState,
    #[serde(deserialize_with = "text")]
    pub notes: String,
    pub prerelease: bool,
    pub released_at: Option<f64>,
    /// Why it is not verified, or why it is blocked, worded for players; empty when verified.
    #[serde(deserialize_with = "list")]
    pub reasons: Vec<String>,
    /// When it is verified by itself if nothing changes.
    pub check_ends_at: Option<f64>,
    pub scan: Option<Scan>,
    #[serde(deserialize_with = "list")]
    pub assets: Vec<Asset>,
    /// Rebuilt under the same tag: the checksums are of the newest build the site saw.
    pub rolling: bool,
    pub upstream_url: Option<String>,
}

/// An app in the release status feed (`/release-status`): enough to match a repository to its entry.
#[derive(Clone, PartialEq, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleaseStatus {
    #[serde(deserialize_with = "text")]
    pub id: String,
    #[serde(deserialize_with = "text")]
    pub slug: String,
    #[serde(deserialize_with = "text")]
    pub provider: String,
    pub repository: Option<String>,
    pub verified: Option<Verified>,
    /// The newest release the developer published, verified or not.
    pub latest_upstream: Option<Verified>,
}
