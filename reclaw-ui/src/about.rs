//! What this build is: the version, the commit it was built from, and the profile. Set once at start by the program (the build script
//! that knows the commit is the program's, not the library's), read by the About page. It is fixed for the life of the process, which
//! is why it is a plain `OnceLock` and not part of the store: nothing can change it and nothing needs to be told it changed.
use std::sync::OnceLock;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct BuildInfo {
    pub version: String,
    /// The short commit hash, with `+local-changes` when the tree had uncommitted edits, or `unknown` (a build from an archive).
    pub commit: String,
    /// `debug` or `release`.
    pub profile: String,
}

impl BuildInfo {
    /// What shows when the program did not say.
    pub fn unknown() -> Self {
        Self { version: env!("CARGO_PKG_VERSION").to_string(), commit: "unknown".to_string(), profile: "unknown".to_string() }
    }

    /// "a1b2c3d4e5 (release)": what to compare with the remote after an update.
    pub fn summary(&self) -> String {
        format!("{} ({})", self.commit, self.profile)
    }
}

static BUILD: OnceLock<BuildInfo> = OnceLock::new();

/// Record this build. Only the first call counts.
pub fn set_build(info: BuildInfo) {
    let _ = BUILD.set(info);
}

pub fn build() -> BuildInfo {
    BUILD.get().cloned().unwrap_or_else(BuildInfo::unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_summary_names_the_commit_and_the_profile() {
        let info = BuildInfo { version: "0.1.0".into(), commit: "a1b2c3d4e5+local-changes".into(), profile: "debug".into() };
        assert_eq!(info.summary(), "a1b2c3d4e5+local-changes (debug)");
    }

    #[test]
    fn an_unset_build_is_unknown_not_empty() {
        assert_eq!(BuildInfo::unknown().summary(), "unknown (unknown)");
    }
}
