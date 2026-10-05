//! The diagnostics report: one text file with what a person helping needs to see and would otherwise have to ask for, one question
//! at a time. Versions, the system and session, the settings that matter to the network, what each service the program needs
//! answers from this machine, whether each has a token (never the token), and the end of the log. Everything passes through the
//! log's redaction on the way out, so a credential in a proxy address or an error message cannot get into it.
use std::{fmt::Write as _, thread, time::SystemTime};

use reclaw_net::{Provider, TokenState, diagnose::HOSTS};
use reclaw_ui::notices::Notice;

use super::Host;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const GIT: &str = env!("RECLAW_GIT_SHA");
pub const PROFILE: &str = env!("RECLAW_PROFILE");
/// How many lines of the log go into the report.
const LOG_LINES: usize = 200;

/// Names that hold nothing secret and say how the session and the network are set up.
const ENVIRONMENT: &[&str] = &[
    "XDG_SESSION_TYPE",
    "XDG_CURRENT_DESKTOP",
    "XDG_SESSION_DESKTOP",
    "WAYLAND_DISPLAY",
    "DISPLAY",
    "HTTPS_PROXY",
    "https_proxy",
    "HTTP_PROXY",
    "http_proxy",
    "ALL_PROXY",
    "NO_PROXY",
    "SSL_CERT_FILE",
    "SteamDeck",
    "SteamOS",
    "container",
];

/// What goes into a report, already gathered.
#[derive(Default)]
pub(super) struct Facts {
    pub unix_time: u64,
    pub system: Vec<(String, String)>,
    pub folders: Vec<(String, String)>,
    pub environment: Vec<(String, String)>,
    pub tokens: Vec<(String, String)>,
    pub services: Vec<(String, String)>,
    pub catalog: Vec<String>,
    pub log: Vec<String>,
}

/// The report as text. Pure, so its shape is tested.
pub(super) fn render(facts: &Facts) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Reclaw diagnostics\n==================\nWritten at unix time {}. Contains no tokens.\n", facts.unix_time);
    let table = |out: &mut String, title: &str, rows: &[(String, String)]| {
        let _ = writeln!(out, "{title}\n{}", "-".repeat(title.len()));
        for (key, value) in rows {
            let _ = writeln!(out, "{key}: {value}");
        }
        out.push('\n');
    };
    table(&mut out, "System", &facts.system);
    table(&mut out, "Folders", &facts.folders);
    table(&mut out, "Environment", &facts.environment);
    table(&mut out, "Access tokens", &facts.tokens);
    table(&mut out, "Services, asked from this machine", &facts.services);
    let _ = writeln!(out, "Catalog\n-------");
    for line in &facts.catalog {
        let _ = writeln!(out, "{line}");
    }
    let _ = writeln!(out, "\nEnd of the log ({} lines)\n------------------------", facts.log.len());
    for line in &facts.log {
        let _ = writeln!(out, "{line}");
    }
    reclaw_log::scrub(&out).into_owned()
}

fn os_release() -> Option<String> {
    let text = std::fs::read_to_string("/etc/os-release").ok()?;
    text.lines().find_map(|l| l.strip_prefix("PRETTY_NAME=")).map(|v| v.trim_matches('"').to_string())
}

fn environment() -> Vec<(String, String)> {
    let mut rows: Vec<(String, String)> = ENVIRONMENT.iter().filter_map(|n| std::env::var(n).ok().map(|v| ((*n).to_string(), v))).collect();
    let mut ours: Vec<(String, String)> = std::env::vars()
        .filter(|(k, _)| k.starts_with("RECLAW_"))
        .map(|(k, v)| {
            let secret = ["TOKEN", "SECRET", "PASSWORD", "KEY"].iter().any(|w| k.contains(w));
            (k, if secret { "(set)".to_string() } else { v })
        })
        .collect();
    ours.sort();
    rows.extend(ours);
    if rows.is_empty() {
        rows.push(("(none of the usual ones are set)".to_string(), String::new()));
    }
    rows
}

impl Host {
    /// Gather the report and write it beside the log. In the background: asking every service takes a few seconds.
    pub(super) fn save_report(&self) {
        let Some(dir) = self.inner.logs_dir.clone() else {
            self.tell(Notice::problem("No place to save the report", "No folder for Reclaw's files was found", vec![]));
            return;
        };
        self.tell(Notice::note("Collecting diagnostics", "This takes a few seconds", vec![]));
        let host = self.clone();
        let started = thread::Builder::new().name("reclaw-diagnostics".into()).spawn(move || {
            let facts = host.gather(&dir);
            let unix = facts.unix_time;
            let path = dir.join(format!("diagnostics-{unix}.txt"));
            let written = std::fs::create_dir_all(&dir).and_then(|()| std::fs::write(&path, render(&facts)));
            match written {
                Ok(()) => {
                    tracing::info!(path = %path.display(), "diagnostics report saved");
                    host.tell(Notice::note(
                        "Diagnostics saved",
                        &path.display().to_string(),
                        vec!["Attach this file when you ask for help. It contains no tokens.".to_string()],
                    ));
                }
                Err(error) => {
                    tracing::error!(path = %path.display(), %error, "the diagnostics report could not be saved");
                    host.tell(Notice::problem(
                        "The diagnostics report could not be saved",
                        &error.to_string(),
                        vec![format!("Tried {}", path.display())],
                    ));
                }
            }
        });
        if let Err(error) = started {
            tracing::error!(%error, "the diagnostics thread did not start");
            self.tell(Notice::problem("The diagnostics report could not be started", &error.to_string(), vec![]));
        }
    }

    fn gather(&self, logs: &std::path::Path) -> Facts {
        let kv = |k: &str, v: String| (k.to_string(), v);
        let mut facts =
            Facts { unix_time: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs()), ..Facts::default() };
        facts.system = vec![
            kv("Reclaw", format!("{VERSION} (commit {GIT}, {PROFILE} build)")),
            kv("system", format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)),
            kv("distribution", os_release().unwrap_or_else(|| "unknown".to_string())),
            kv(
                "kernel",
                std::fs::read_to_string("/proc/sys/kernel/osrelease")
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string()),
            ),
        ];
        facts.folders = vec![kv("logs", logs.display().to_string())];
        facts.environment = environment();
        {
            let status = self.inner.tokens.status();
            for provider in Provider::ALL {
                let state = self.inner.net.as_ref().map_or("no network layer", |n| match n.token_state(provider) {
                    TokenState::None => "none sent",
                    TokenState::Active => "sent",
                    TokenState::Rejected => "refused by the service, not sent",
                });
                let source = format!("{:?}", status.of(provider).source);
                facts.tokens.push(kv(provider.label(), format!("{source}; {state}; {}", status.of(provider).short(provider))));
            }
        }
        if let Some(net) = &self.inner.net {
            // Every service at once: the slowest one decides how long this takes, not the sum.
            let probes: Vec<(String, String)> = thread::scope(|scope| {
                let workers: Vec<_> = HOSTS.iter().map(|(url, what)| (what, scope.spawn(move || net.probe(url)))).collect();
                workers
                    .into_iter()
                    .map(|(what, worker)| {
                        let summary = worker
                            .join()
                            .map_or_else(|_| "the probe stopped unexpectedly".to_string(), |p| format!("{}: {}", p.url, p.summary()));
                        ((*what).to_string(), summary)
                    })
                    .collect()
            });
            facts.services = probes;
        } else {
            facts.services = vec![kv("network", "the network layer did not start".to_string())];
        }
        {
            let state = self.state();
            facts.catalog.push(format!("{} apps in the catalog, {} in the library", state.catalog.len(), state.library.len()));
            facts.catalog.push(format!("status: {:?}, stale: {}", state.status.phase, state.status.stale));
            facts.catalog.extend(state.status.problems.iter().map(|p| format!("problem: {p}")));
        }
        facts.log = match self.inner.logging.as_ref().and_then(|l| l.file().map(std::path::Path::to_path_buf)) {
            Some(file) => reclaw_log::tail(&file, LOG_LINES).unwrap_or_else(|e| vec![format!("(the log could not be read: {e})")]),
            None => vec!["(no log file is being written)".to_string()],
        };
        facts
    }
}

#[cfg(test)]
mod tests;
