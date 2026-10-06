//! Installing, updating and removing apps. A job is one thread working through `reclaw-install`'s steps and reporting each to the
//! screens as activity events; what is installed is read from the folders themselves, so there is no second record to drift.
//!
//! * `paths`: where an app's folder is (pure)
//! * `jobs`: the install and update flow, cancelling, and how a job ends
//! * `actions`: uninstall, open the folder, check for an update, verify
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use reclaw_install::Installer;
use reclaw_net::Cancel;
use reclaw_ui::activity::ActivityId;

mod actions;
mod jobs;
pub mod paths;

/// What the host works with to install: the installer, and where things are on this machine.
pub struct Installs {
    pub(super) installer: Option<Installer>,
    pub(super) platform: reclaw_install::Platform,
    pub(super) home: Option<PathBuf>,
    jobs: Mutex<HashMap<u32, Running>>,
    next_activity: AtomicU64,
}

/// A job that is working now.
struct Running {
    activity: ActivityId,
    cancel: Cancel,
}

impl Installs {
    pub fn new(installer: Option<Installer>, platform: reclaw_install::Platform, home: Option<PathBuf>) -> Self {
        Self { installer, platform, home, jobs: Mutex::new(HashMap::new()), next_activity: AtomicU64::new(1) }
    }

    fn jobs(&self) -> std::sync::MutexGuard<'_, HashMap<u32, Running>> {
        self.jobs.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub(crate) fn next_activity(&self) -> ActivityId {
        self.next_activity.fetch_add(1, Ordering::Relaxed)
    }

    /// Whether this app has a job running.
    pub fn is_busy(&self, app: u32) -> bool {
        self.jobs().contains_key(&app)
    }

    /// Stop the job that owns this activity, if one is running. The partial download is kept for a later try.
    pub fn cancel(&self, activity: ActivityId) -> bool {
        match self.jobs().values().find(|job| job.activity == activity) {
            Some(job) => {
                job.cancel.cancel();
                true
            }
            None => false,
        }
    }

    /// The places that are never deleted by an uninstall: the home folder and where Reclaw keeps its own files.
    pub(super) fn protected(&self, extra: &[PathBuf]) -> Vec<PathBuf> {
        self.home.iter().cloned().chain(extra.iter().cloned()).collect()
    }
}

impl crate::host::Host {
    /// Create the catalog's marker files (`filesToAdd`) in an app's folder, as Quiver does after every install. `existing`: the
    /// folder already held an install, so the game may have kept its settings somewhere else until now (a recomp without
    /// `portable.txt` uses `~/.config/<its name>`), and the person is told where their earlier saves are.
    pub(crate) fn add_markers(&self, title: &str, folder: &Path, names: &[String], existing: bool) {
        match reclaw_install::layout::add_marker_files(folder, names) {
            Ok(created) if !created.is_empty() => {
                tracing::info!(app = title, folder = %shown(folder), files = ?created, existing, "added the files the catalog asks for");
                if existing {
                    self.tell(reclaw_ui::notices::Notice::note(
                        &format!("{title} now keeps its settings in its own folder"),
                        &format!("Reclaw added {}, as the catalog asks", created.join(", ")),
                        vec![
                            format!("The folder is {}.", shown(folder)),
                            "Settings and saves from before stay where the game kept them (for the recomps, a folder named after the game in ~/.config); copy them into this folder to keep using them.".to_string(),
                        ],
                    ));
                }
            }
            Ok(_) => {}
            Err(error) => {
                tracing::warn!(app = title, folder = %shown(folder), %error, "the files the catalog asks for could not be created");
                self.tell(reclaw_ui::notices::Notice::problem(
                    &format!("{title} may not find its mods"),
                    "A file the catalog asks for could not be created in its folder",
                    vec![error.to_string(), format!("Create {} in {} yourself.", names.join(", "), shown(folder))],
                ));
            }
        }
    }
}

/// `path` as a string for a message.
fn shown(path: &Path) -> String {
    path.display().to_string()
}

#[cfg(test)]
pub(crate) mod tests;
