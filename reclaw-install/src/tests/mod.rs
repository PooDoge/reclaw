//! Tests of the pure rules: file names, platform matching, asset choice, release parsing and selection.
mod matcher;
mod names;
mod policy;
mod release;

use crate::release::{Asset, Release};

/// A release with these file names (a made-up address each).
pub fn release_with(tag: &str, prerelease: bool, names: &[&str]) -> Release {
    Release {
        tag: tag.to_string(),
        prerelease,
        assets: names
            .iter()
            .map(|n| Asset { name: (*n).to_string(), url: format!("https://example.test/dl/{tag}/{n}"), size: None, sha256: None })
            .collect(),
        ..Release::default()
    }
}
