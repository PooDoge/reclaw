//! Mods for the apps in the library: listing what Thunderstore and GameBanana offer for an app, and installing, updating and
//! removing them in the app's mods folder, with a record of every file each mod put there. No UI types; the program's host calls
//! it from worker threads and reports what happens.
//!
//! * `source`: which site and which page of it an app's catalog entry points at (pure)
//! * `package`: one shape for a listed mod and for what an install downloads
//! * `thunderstore`, `gamebanana`: each site's addresses and answers (pure)
//! * `client`: asking the sites through the program's one HTTP client
//! * `sidecar`: `.quiver-mods.json`, the record of installed mods, in Quiver's format
//! * `plan`: where each file of a mod goes, and which would replace another mod's (pure)
//! * `install`: the install and the removal, dependencies first; `error`: why anything here failed
pub mod client;
pub mod error;
pub mod gamebanana;
pub mod install;
pub mod package;
pub mod plan;
pub mod sidecar;
pub mod source;
pub mod thunderstore;

pub use client::{Endpoints, ModSites};
pub use error::ModError;
pub use install::{ModInstaller, Stage, Step, Target, install_with_dependencies, uninstall};
pub use package::{Download, Package, Page, Sort};
pub use sidecar::{Document, Record};
pub use source::{Provider, Source};

/// Whether `latest` is an update over `installed`. Versions that read as numbers compare as the rest of Reclaw compares release
/// tags; two that do not (GameBanana allows any text) are an update when they differ, as in Quiver: offering to reinstall the
/// same thing is better than hiding a new one. Unknown on either side is no update.
pub fn is_update(installed: &str, latest: &str) -> bool {
    use reclaw_games::version;
    let (installed, latest) = (installed.trim(), latest.trim());
    if installed.is_empty() || latest.is_empty() {
        return false;
    }
    let numeric = |v: &str| v.trim_start_matches(['v', 'V']).starts_with(|c: char| c.is_ascii_digit());
    if numeric(installed) && numeric(latest) {
        !version::equivalent(installed, latest) && version::is_newer(latest, installed)
    } else {
        !installed.eq_ignore_ascii_case(latest)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn an_update_is_a_newer_version_known_on_both_sides() {
        assert!(super::is_update("1.0.0", "1.2.0"));
        assert!(super::is_update("v1.0", "1.0.1"));
        assert!(!super::is_update("1.2.0", "1.2.0"));
        assert!(!super::is_update("1.2.0", "1.1.9"));
        assert!(!super::is_update("", "1.0"));
        assert!(!super::is_update("1.0", ""));
        assert!(super::is_update("Beta", "Final"));
        assert!(!super::is_update("final", "Final"));
    }
}
