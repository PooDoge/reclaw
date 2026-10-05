//! The program's log: where it is, how much goes in it, and what never does. Every crate writes with the `tracing` macros and knows
//! nothing about files; the binary calls [`init`] once, near the start, and the messages of the toolkit and the HTTP stack land
//! in the same place.
//!
//! * `level`: [`LogLevel`] (the Settings choice) and which targets count as ours
//! * `rotate`: the file, moved aside when it is full (`reclaw.log`, `reclaw.1.log` ...)
//! * `sink`: events collected whole and cleaned on their way out
//! * `redact`, `secret`: [`scrub`], [`register_secret`] and [`Secret`]: credentials are never written, whatever shape they have
//! * `panic`: a panic is logged, with its stack, before anything else happens
//! * `tail`: the last lines of the file, for the diagnostics report
//!
//! Why this and not `println!`: a launcher is usually started from a menu entry, where standard error goes nowhere. A failure that
//! is only printed is a failure nobody can look at afterwards. The rule the rest of the workspace follows (and `tests/repo_hygiene.rs`
//! checks): no `eprintln!` outside this crate; say it with `tracing`, with the facts that would be asked for next (which host, which
//! file, which attempt, how long) as fields.
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use tracing_subscriber::{Registry, filter::Targets, fmt, layer::SubscriberExt, reload, util::SubscriberInitExt};

mod level;
mod panic;
mod redact;
mod rotate;
mod secret;
mod sink;
mod tail;

pub use level::{LogLevel, OUR_TARGETS};
pub use redact::scrub;
pub use rotate::RotatingFile;
pub use secret::{REDACTED, Secret, forget_secret, register_secret};
pub use tail::tail;

/// A file is moved aside at this size; with [`KEEP`] files that bounds the log at about ten megabytes.
const MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const KEEP: usize = 5;

/// What to set up.
#[derive(Clone, Debug)]
pub struct LogConfig {
    /// The folder of `reclaw.log`. `None` writes no file (nowhere to put one was found).
    pub dir: Option<PathBuf>,
    pub level: LogLevel,
    /// A filter written by hand (`RECLAW_LOG`); replaces the level when it parses.
    pub filter: Option<String>,
    /// Also write to standard error: for a program started from a terminal.
    pub stderr: bool,
}

/// What [`init`] made. Keep it for as long as the program runs: it changes the level and knows where the file is.
pub struct Logging {
    handle: Option<reload::Handle<Targets, Registry>>,
    dir: Option<PathBuf>,
    file: Option<PathBuf>,
    problems: Vec<String>,
    overridden: bool,
}

impl Logging {
    /// Change how much is written, now. Does nothing while `RECLAW_LOG` decides.
    pub fn set_level(&self, level: LogLevel) {
        if self.overridden {
            tracing::debug!(target: "reclaw_log", "the log level stays as RECLAW_LOG set it");
            return;
        }
        if let Some(handle) = &self.handle
            && let Err(error) = handle.reload(level.targets())
        {
            tracing::warn!(target: "reclaw_log", "the log level could not be changed: {error}");
        }
    }

    /// The folder holding the log files.
    pub fn dir(&self) -> Option<&Path> {
        self.dir.as_deref()
    }

    /// The file being written.
    pub fn file(&self) -> Option<&Path> {
        self.file.as_deref()
    }

    /// Things that could not be set up (no folder, a filter that did not parse, a logger already present). The program runs
    /// either way; tell the user once.
    pub fn problems(&self) -> &[String] {
        &self.problems
    }

    /// Whether `RECLAW_LOG` is deciding the level.
    pub fn is_overridden(&self) -> bool {
        self.overridden
    }
}

/// Install the logger. Never fails: what could not be done is in [`Logging::problems`], and the program runs without that part.
pub fn init(config: LogConfig) -> Logging {
    let mut problems = Vec::new();
    let wanted = config.filter.as_deref().map(str::trim).filter(|f| !f.is_empty());
    let (targets, overridden) = match wanted.map(level::parse_override) {
        Some(Ok(targets)) => (targets, true),
        Some(Err(why)) => {
            problems.push(why);
            (config.level.targets(), false)
        }
        None => (config.level.targets(), false),
    };
    let (filter, handle) = reload::Layer::new(targets);

    let file = config.dir.as_deref().and_then(|dir| match RotatingFile::open(dir, "reclaw", MAX_FILE_BYTES, KEEP) {
        Ok(file) => Some(file),
        Err(error) => {
            problems.push(format!("The log file could not be opened in {}: {error}", dir.display()));
            None
        }
    });
    let path = file.as_ref().map(RotatingFile::current_path);
    let file_layer =
        file.map(|file| fmt::layer().with_ansi(false).with_thread_names(true).with_writer(sink::FileDest(Arc::new(Mutex::new(file)))));
    let stderr_layer = config.stderr.then(|| fmt::layer().with_ansi(false).with_thread_names(true).with_writer(sink::StderrDest));

    match tracing_subscriber::registry().with(filter).with(file_layer).with(stderr_layer).try_init() {
        Ok(()) => {
            panic::install();
            Logging { handle: Some(handle), dir: config.dir, file: path, problems, overridden }
        }
        Err(error) => {
            problems.push(format!("Another logger was already installed, so Reclaw's log file is not written: {error}"));
            Logging { handle: None, dir: config.dir, file: None, problems, overridden }
        }
    }
}

/// For tests: send messages to the test harness's captured output, so a failing test shows what the code logged. Safe to call from
/// every test; only the first call does anything.
pub fn init_for_tests() {
    let _ = tracing_subscriber::fmt().with_test_writer().with_max_level(tracing::Level::DEBUG).try_init();
}
