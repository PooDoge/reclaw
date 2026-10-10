//! What the catalog's website says about a project, beyond what the catalog format carries: when it was added and last released,
//! what players said about how it runs, what kind of project it is, where it runs, how much AI wrote it, and which of its releases
//! the site verified. The Catalog sorts and filters by these and its cards show them. Plain data; the site's own types stay in
//! `reclaw-catalog`.
use serde::{Deserialize, Serialize};

/// How much of a project AI wrote, as the site judged it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum AiUse {
    /// None found, or the site has not said.
    #[default]
    None,
    Assisted,
    Generated,
}

/// What players said about how it runs: how many said each.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Ratings {
    pub runs: u32,
    pub issues: u32,
    pub broken: u32,
}

impl Ratings {
    pub fn total(self) -> u32 {
        self.runs.saturating_add(self.issues).saturating_add(self.broken)
    }

    /// The order "Top rated" uses, Quiver's player-feedback score: (runs well + half of has issues + 1) / (everyone + 2), then how
    /// many said anything. The +1/+2 pulls a few votes toward the middle, so one happy player does not outrank fifty mostly happy ones,
    /// and an unrated project sits at one half, above one with only bad reports.
    pub fn rank(self) -> (u64, u32) {
        let (runs, issues, total) = (u64::from(self.runs), u64::from(self.issues), u64::from(self.total()));
        // In millionths, doubled to keep the half vote whole.
        ((runs * 2 + issues + 2) * 1_000_000 / ((total + 2) * 2), self.total())
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct Listing {
    /// Unix seconds when the site added it.
    pub added_at: Option<u64>,
    /// Unix seconds of its newest release upstream.
    pub updated_at: Option<u64>,
    pub ratings: Ratings,
    /// `port`, `tool`, `emulator` or `game`; empty when the site does not say.
    pub project_type: String,
    /// Where it runs, by the site's ids: `windows`, `linux`, `macos`, `android`, `ios`.
    pub runs_on: Vec<String>,
    pub ai: AiUse,
    /// The release the site verified: what Reclaw installs and updates to.
    pub verified: Option<String>,
    /// The newest release upstream, verified or not.
    pub latest: Option<String>,
    /// The original games it plays.
    pub based_on: Vec<String>,
}

impl Listing {
    /// Added in the last thirty days, as the site's NEW badge says.
    pub fn is_new(&self, now: u64) -> bool {
        self.added_at.is_some_and(|added| now.saturating_sub(added) < 30 * 24 * 3600)
    }

    /// A newer release than the verified one is out, not verified yet.
    pub fn unverified_newer(&self) -> Option<&str> {
        let latest = self.latest.as_deref()?;
        match self.verified.as_deref() {
            Some(verified) if !crate::version::is_newer(latest, verified) => None,
            _ => Some(latest),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_rated_weighs_the_share_and_then_the_number_of_players() {
        let r = |runs, issues, broken| Ratings { runs, issues, broken };
        assert!(r(9, 1, 0).rank() > r(1, 1, 0).rank());
        assert!(r(40, 0, 0).rank() > r(1, 0, 0).rank(), "same share, more players");
        assert!(r(49, 1, 0).rank() > r(1, 0, 0).rank(), "fifty mostly happy players beat one happy one");
        assert!(r(0, 0, 0).rank() > r(0, 0, 1).rank(), "unrated sits above a single bad report");
        assert_eq!(r(0, 0, 0).total(), 0);
    }

    #[test]
    fn new_is_thirty_days_and_unverified_is_only_a_newer_one() {
        let listing = Listing { added_at: Some(1_000), latest: Some("v2".into()), verified: Some("v1".into()), ..Default::default() };
        assert!(listing.is_new(1_000 + 29 * 24 * 3600) && !listing.is_new(1_000 + 31 * 24 * 3600));
        assert_eq!(listing.unverified_newer(), Some("v2"));
        let current = Listing { latest: Some("v1".into()), verified: Some("v1".into()), ..Default::default() };
        assert_eq!(current.unverified_newer(), None);
        let none_verified = Listing { latest: Some("v1".into()), ..Default::default() };
        assert_eq!(none_verified.unverified_newer(), Some("v1"));
    }
}
