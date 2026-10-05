//! Updating a development copy from the git checkout it was built from: run `scripts/update.sh` (the same script a terminal uses),
//! pass its output to the log line by line, and turn what happened into a notice that says what to do next.
//!
//! This exists for the loop of building and trying on a machine other than the one the code is written on. A copy that was not built
//! from a checkout (no source folder was stamped into it, or it has moved) says so instead of guessing.
//!
//! The script, not this code, holds the safety rules: it follows the branch the checkout is on, refuses local changes and diverged
//! histories, and never resets or rebases. The program cannot restart itself: a window that vanishes and reappears while a person
//! is looking at it is worse than a message saying to restart, and the old binary keeps working until then.
use std::{
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{Arc, Mutex, PoisonError, atomic::Ordering},
    thread,
};

use reclaw_ui::notices::Notice;

use super::{Host, Inner};

/// How many lines of the script's output are kept for a failure notice.
const KEPT_LINES: usize = 40;

/// Where this copy was built from, and how to update it.
#[derive(Clone, Debug)]
pub struct UpdateSource {
    pub dir: PathBuf,
    pub script: PathBuf,
    /// `debug` or `release`: the profile this copy was built with, so the rebuild produces the binary that is running.
    pub profile: String,
    /// The executable that is running, so the script can replace an installed copy.
    pub running_exe: Option<PathBuf>,
}

impl UpdateSource {
    /// The checkout stamped into the build (`dir`), if it is still there and still has the script. `None` otherwise.
    pub fn detect(dir: &str, profile: &str) -> Option<Self> {
        let dir = PathBuf::from(dir.trim());
        let script = dir.join("scripts").join("update.sh");
        (!dir.as_os_str().is_empty() && dir.join(".git").exists() && script.is_file()).then(|| Self {
            dir,
            script,
            profile: if profile == "debug" { "debug" } else { "release" }.to_string(),
            running_exe: std::env::current_exe().ok(),
        })
    }
}

/// The script's last line: `RECLAW_UPDATE: <result> <old> <new> <binary>`.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Marker {
    pub result: String,
    pub old: String,
    pub new: String,
    pub binary: Option<String>,
}

pub(super) fn parse_marker(line: &str) -> Option<Marker> {
    let mut parts = line.strip_prefix("RECLAW_UPDATE: ")?.split_whitespace();
    let (result, old, new) = (parts.next()?.to_string(), parts.next()?.to_string(), parts.next()?.to_string());
    let binary = parts.next().filter(|b| *b != "none").map(str::to_string);
    Some(Marker { result, old, new, binary })
}

/// What a run of the script came to.
#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct Finished {
    /// The exit code; `None` when the script could not be started or was killed.
    pub code: Option<i32>,
    pub marker: Option<Marker>,
    /// Why it could not be started, if it could not.
    pub spawn_error: Option<String>,
    /// The new commits, as the script listed them.
    pub commits: Vec<String>,
    /// The end of the output, both streams, for a failure.
    pub tail: Vec<String>,
}

/// The notice for an outcome. Pure, so every exit code of the script is tested.
pub(super) fn notice_for(done: &Finished) -> Notice {
    let tail = || done.tail.iter().rev().take(15).rev().cloned().collect::<Vec<_>>();
    if let Some(error) = &done.spawn_error {
        return Notice::problem(
            "The update could not start",
            error,
            vec!["It needs bash and git. Run scripts/update.sh in a terminal to see more.".to_string()],
        );
    }
    let result = done.marker.as_ref().map(|m| m.result.as_str());
    match (done.code, result) {
        (Some(0), Some("updated")) => {
            let m = done.marker.as_ref();
            let mut details = done.commits.clone();
            if let Some(binary) = m.and_then(|m| m.binary.as_ref()) {
                details.push(format!("Built: {binary}"));
            }
            Notice::note(
                "Reclaw updated",
                &format!(
                    "{} to {}. Quit and start Reclaw again to use it.",
                    m.map_or("?", |m| m.old.as_str()),
                    m.map_or("?", |m| m.new.as_str())
                ),
                details,
            )
        }
        (Some(0), Some("up-to-date")) => {
            Notice::note("Reclaw is up to date", &format!("Already at {}", done.marker.as_ref().map_or("?", |m| m.new.as_str())), vec![])
        }
        (Some(0), Some("ahead")) => Notice::note("Nothing to update", "This checkout has commits the remote does not", vec![]),
        (Some(0), _) => Notice::note("The update finished", "It did not say what it did", tail()),
        (Some(4), _) => Notice::problem("The update stopped: you have local changes", "Nothing was changed", tail()),
        (Some(3), _) => Notice::problem(
            "The update stopped: the histories differ",
            "Nothing was changed",
            std::iter::once(
                "Your checkout and the remote have both moved on. Look at it with: git log --oneline --graph --all".to_string(),
            )
            .chain(tail())
            .collect(),
        ),
        (Some(5), _) => Notice::problem("The update could not reach the remote", "Check the network connection", tail()),
        (Some(6), _) => Notice::problem("The build failed", "The new source is in place; fix the error and update again", tail()),
        (Some(2), _) => Notice::problem("The update cannot run here", "This checkout cannot be updated", tail()),
        (Some(code), _) => Notice::problem("The update failed", &format!("The script exited with {code}"), tail()),
        (None, _) => Notice::problem("The update was interrupted", "The script was stopped before it finished", tail()),
    }
}

/// Ends the "an update is running" state however the thread ends.
struct Running(Arc<Inner>);

impl Drop for Running {
    fn drop(&mut self) {
        self.0.updating.store(false, Ordering::SeqCst);
    }
}

fn run_script(source: &UpdateSource) -> Finished {
    let mut command = Command::new("bash");
    command
        .arg(&source.script)
        .args(["--profile", &source.profile])
        .current_dir(&source.dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(exe) = &source.running_exe {
        command.env("RECLAW_RUNNING_EXE", exe);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => return Finished { spawn_error: Some(format!("bash could not be started: {error}")), ..Finished::default() },
    };
    let collected = Arc::new(Mutex::new((Vec::<String>::new(), Vec::<String>::new(), None::<Marker>)));
    let pipe = |stream: Option<Box<dyn std::io::Read + Send>>, is_stdout: bool| {
        let collected = collected.clone();
        thread::spawn(move || {
            let Some(stream) = stream else { return };
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                if is_stdout {
                    tracing::info!(target: "reclaw_app::update", "{line}");
                } else {
                    tracing::warn!(target: "reclaw_app::update", "{line}");
                }
                let mut c = collected.lock().unwrap_or_else(PoisonError::into_inner);
                if let Some(marker) = parse_marker(&line) {
                    c.2 = Some(marker);
                } else if is_stdout && line.starts_with("    ") {
                    c.1.push(line.trim().to_string());
                }
                let tail = &mut c.0;
                tail.push(line);
                if tail.len() > KEPT_LINES {
                    tail.remove(0);
                }
            }
        })
    };
    let out = pipe(child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), true);
    let err = pipe(child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>), false);
    let status = child.wait();
    // Both readers end when the pipes close; wait for them so no line is lost.
    let _ = (out.join(), err.join());
    let (tail, commits, marker) = std::mem::take(&mut *collected.lock().unwrap_or_else(PoisonError::into_inner));
    Finished { code: status.ok().and_then(|s| s.code()), marker, spawn_error: None, commits, tail }
}

impl Host {
    /// Update this copy from its source checkout, in the background.
    pub(super) fn update_sources(&self) {
        let Some(source) = self.inner.update.clone() else {
            tracing::info!("an update was asked for, but this copy was not built from a source checkout");
            self.tell(Notice::note(
                "This copy cannot update itself",
                "It was not built from a git checkout",
                vec![
                    "Build from a clone: git clone https://github.com/PooDoge/reclaw, then scripts/bazzite-build.sh --install (Bazzite) or cargo build --release.".to_string(),
                    "After that, Update from source (or scripts/update.sh) brings it up to date.".to_string(),
                ],
            ));
            return;
        };
        if self.inner.updating.swap(true, Ordering::SeqCst) {
            self.tell(Notice::note("An update is already running", "Wait for it to finish", vec![]));
            return;
        }
        tracing::info!(dir = %source.dir.display(), profile = %source.profile, "updating from the source checkout");
        self.tell(Notice::note(
            "Updating Reclaw",
            "Fetching the newest commits, then building",
            vec![format!(
                "From {}. A build after a big change takes several minutes; Reclaw keeps working meanwhile.",
                source.dir.display()
            )],
        ));
        let (host, running) = (self.clone(), Running(self.inner.clone()));
        let started = thread::Builder::new().name("reclaw-update".into()).spawn(move || {
            let _running = running;
            let done = run_script(&source);
            tracing::info!(code = ?done.code, result = ?done.marker.as_ref().map(|m| &m.result), "the update script finished");
            host.tell(notice_for(&done));
        });
        if let Err(error) = started {
            self.inner.updating.store(false, Ordering::SeqCst);
            tracing::error!(%error, "the update thread did not start");
            self.tell(Notice::problem("The update could not start", "A background thread did not start", vec![error.to_string()]));
        }
    }
}

#[cfg(test)]
mod tests;
