//! How much is written, and which messages count as ours. The file is for finding out what went wrong, so the default keeps our own
//! messages at `info` and everything else (the toolkit, the HTTP stack) at `warn`.
use tracing::Level;
use tracing_subscriber::filter::Targets;

/// The `tracing` targets of this program's own crates (a target is the module path, which starts with the crate's name). A crate
/// added to the workspace must be listed or its messages count as a dependency's; `tests/repo_hygiene.rs` checks the list.
pub const OUR_TARGETS: &[&str] = &[
    "reclaw",
    "reclaw_app",
    "reclaw_catalog",
    "reclaw_config",
    "reclaw_games",
    "reclaw_input",
    "reclaw_install",
    "reclaw_log",
    "reclaw_media",
    "reclaw_mods",
    "reclaw_net",
    "reclaw_runtime",
    "reclaw_sync",
    "reclaw_ui",
];

/// The Settings choice, in this order (a saved choice is a position: append, never reorder).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LogLevel {
    /// Only what went wrong.
    Problems,
    /// What happened, in one line each: a catalog refresh, a download, a setting that could not be saved.
    #[default]
    Normal,
    /// Every request and every decision, and the HTTP stack's own notes. For chasing a problem; the file grows fast.
    Detailed,
}

impl LogLevel {
    pub const ALL: [LogLevel; 3] = [Self::Problems, Self::Normal, Self::Detailed];

    /// From a saved choice index; anything unknown is the default.
    pub fn from_index(index: usize) -> Self {
        Self::ALL.get(index).copied().unwrap_or_default()
    }

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|l| *l == self).unwrap_or(1)
    }

    /// (our crates, everyone else's).
    fn levels(self) -> (Level, Level) {
        match self {
            Self::Problems => (Level::WARN, Level::ERROR),
            Self::Normal => (Level::INFO, Level::WARN),
            Self::Detailed => (Level::DEBUG, Level::INFO),
        }
    }

    pub(crate) fn targets(self) -> Targets {
        let (ours, theirs) = self.levels();
        OUR_TARGETS.iter().fold(Targets::new().with_default(theirs), |targets, name| targets.with_target(*name, ours))
    }
}

/// A filter written by hand (`RECLAW_LOG=reclaw_net=trace,warn`), or why it cannot be used.
pub(crate) fn parse_override(text: &str) -> Result<Targets, String> {
    text.parse::<Targets>().map_err(|e| format!("RECLAW_LOG={text:?} is not a log filter ({e}); using the Settings level"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_choice_maps_back_and_an_unknown_one_is_normal() {
        for level in LogLevel::ALL {
            assert_eq!(LogLevel::from_index(level.index()), level);
        }
        assert_eq!(LogLevel::from_index(99), LogLevel::Normal);
    }

    #[test]
    fn our_crates_are_more_talkative_than_dependencies() {
        let enabled = |level: LogLevel, target: &str, at: Level| level.targets().would_enable(target, &at);
        // Normal: our info yes, our debug no; a dependency's warn yes, its info no.
        assert!(enabled(LogLevel::Normal, "reclaw_net::net", Level::INFO));
        assert!(!enabled(LogLevel::Normal, "reclaw_net::net", Level::DEBUG));
        assert!(enabled(LogLevel::Normal, "hyper_util::client::legacy", Level::WARN));
        assert!(!enabled(LogLevel::Normal, "hyper_util::client::legacy", Level::INFO));
        // Problems: warnings of ours, errors of theirs.
        assert!(enabled(LogLevel::Problems, "reclaw_sync::catalog", Level::WARN));
        assert!(!enabled(LogLevel::Problems, "reclaw_sync::catalog", Level::INFO));
        assert!(!enabled(LogLevel::Problems, "freya_winit", Level::WARN));
        // Detailed: our debug, their info.
        assert!(enabled(LogLevel::Detailed, "reclaw_ui::store", Level::DEBUG));
        assert!(enabled(LogLevel::Detailed, "reqwest::connect", Level::INFO));
        assert!(!enabled(LogLevel::Detailed, "reqwest::connect", Level::DEBUG));
        // The binary's own target is the bare name.
        assert!(enabled(LogLevel::Normal, "reclaw", Level::INFO));
    }

    #[test]
    fn an_override_parses_or_explains() {
        assert!(parse_override("reclaw_net=trace,warn").is_ok());
        let err = parse_override("reclaw_net=loud").expect_err("not a level");
        assert!(err.contains("RECLAW_LOG") && err.contains("loud"), "{err}");
    }
}
