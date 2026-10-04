//! Reading a settings file written by a different version of Reclaw.
//!
//! Version 1 is the first format. A file with no version (hand-written, or from before versions
//! existed) is read as version 1. A file from a newer Reclaw is read as far as this build
//! understands it, and marked read-only so the next save cannot silently delete what it did not read.
use crate::prefs::Preferences;

/// What reading a file at some version means.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Compat {
    /// Same or older: read it, and a save will upgrade it.
    Current,
    /// Written by a newer Reclaw: read it, never write it.
    Newer { found: u32 },
}

/// Bring `prefs` (just parsed) up to the current format and say whether saving it is safe.
/// Each step converts exactly one version to the next, so a very old file walks the chain.
pub fn upgrade(prefs: &mut Preferences) -> Compat {
    let found = prefs.version;
    if found > Preferences::VERSION {
        return Compat::Newer { found };
    }
    // 0 means "no version key". There is no format 0, so it is already shaped like 1.
    // When format 2 exists, its conversion goes here: `if prefs.version == 1 { ...; prefs.version = 2; }`
    prefs.version = Preferences::VERSION;
    Compat::Current
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_without_a_version_is_read_as_current_and_stamped() {
        let mut p: Preferences = toml::from_str("favorites = [1]").expect("parse");
        assert_eq!(p.version, Preferences::VERSION, "serde's default fills the version");
        p.version = 0;
        assert_eq!(upgrade(&mut p), Compat::Current);
        assert_eq!(p.version, Preferences::VERSION);
    }

    #[test]
    fn a_newer_file_is_flagged_and_left_as_it_was_read() {
        let mut p = Preferences { version: Preferences::VERSION + 3, ..Preferences::default() };
        assert_eq!(upgrade(&mut p), Compat::Newer { found: Preferences::VERSION + 3 });
        assert_eq!(p.version, Preferences::VERSION + 3);
    }
}
