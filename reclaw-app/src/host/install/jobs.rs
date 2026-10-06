use std::{path::Path, thread};

use reclaw_catalog::{AppEntry, RepoSource};
use reclaw_install::{Host as RepoHost, InstallError, Installed, Plan, Request, Resolved, Stage, Step};
use reclaw_net::Cancel;
use reclaw_ui::{
    activity::{ActivityEvent, ActivityId, Changelog, Kind, Stage as UiStage},
    catalog_data::{InstallState, key_of},
    notices::Notice,
    store::AppAction,
};

use super::{Running, paths, shown};
use crate::host::{Host, find, read_only, save_failed};

/// Everything a job needs, copied so the thread owns it.
struct Spec {
    app: u32,
    key: String,
    title: String,
    kind: Kind,
    activity: ActivityId,
    request: Request,
    cancel: Cancel,
    /// The version installed before this job, if any.
    previous: Option<String>,
    /// The catalog's `filesToAdd`, created once the install is in place.
    markers: Vec<String>,
}

/// How a job ended.
enum End {
    Done(Box<Installed>),
    /// The folder already holds the release that would be installed.
    Current(String),
    Failed(InstallError),
    Cancelled,
}

fn host_of(entry: &AppEntry) -> RepoHost {
    if entry.source == RepoSource::Gitlab { RepoHost::GitLab } else { RepoHost::GitHub }
}

fn ui_stage(stage: Stage) -> UiStage {
    match stage {
        Stage::Downloading => UiStage::Downloading,
        Stage::Extracting => UiStage::Extracting,
        Stage::Finishing => UiStage::Finishing,
    }
}

impl Host {
    /// Install an app into `location` (the text of the form), or update it where it is.
    pub(crate) fn start_install(&self, app: u32, location: Option<&str>, prerelease: bool) {
        let installs = &self.inner.installs;
        let Some(installer) = installs.installer.clone() else {
            self.tell(Notice::problem("No network", "The network layer could not start, so nothing can be downloaded", vec![]));
            return;
        };
        if installs.is_busy(app) {
            tracing::debug!(app, "an install or update is already running for this app");
            return;
        }
        let Some((_, entry)) = find(&self.state(), app) else {
            tracing::warn!(app, "an install was asked for an app that is not in the catalog or the library");
            return;
        };
        if entry.is_manual() {
            self.tell(Notice::problem(&format!("{} cannot be downloaded", entry.name), "It has no repository to download from", vec![]));
            return;
        }
        // Where: the form's location for an install, the recorded folder for an update.
        let installed_version = self.installed_version(&entry);
        let typed = location.map(str::trim).filter(|text| !text.is_empty());
        let folder = match (typed, installed_version.is_some()) {
            (Some(text), false) => paths::expand(text, installs.home.as_deref()).and_then(|base| paths::in_location(&entry, &base)),
            // An empty box means the default, whatever an earlier install of this app was recorded under.
            (None, false) => {
                paths::expand(&self.default_location(), installs.home.as_deref()).and_then(|base| paths::in_location(&entry, &base))
            }
            (_, true) => paths::folder_of(&entry, &self.default_location(), installs.home.as_deref()),
        };
        let folder = match folder {
            Ok(folder) => folder,
            Err(problem) => {
                tracing::warn!(app, problem = %problem.message(), "the install location is not usable");
                self.tell(Notice::problem("The install location cannot be used", &problem.message(), vec![problem.hint().to_string()]));
                return;
            }
        };
        // The app belongs to the library from now on, and the library remembers where it went.
        if !self.keep_in_library_at(app, &folder) {
            return;
        }
        let Some((_, entry)) = find(&self.state(), app) else { return };
        let kind = if installed_version.is_some() { Kind::Update } else { Kind::Install };
        let (activity, cancel) = (installs.next_activity(), Cancel::new());
        {
            let mut jobs = installs.jobs();
            if jobs.contains_key(&app) {
                return;
            }
            jobs.insert(app, Running { activity, cancel: cancel.clone() });
        }
        let request = Request {
            host: host_of(&entry),
            repo: entry.repository.trim().to_string(),
            folder,
            platform: installs.platform,
            filter: entry.release_asset_filter.clone(),
            preferred_version: entry.preferred_version.clone(),
            allow_prerelease: prerelease,
            asset: None,
        };
        let spec = Spec {
            app,
            key: key_of(&entry),
            title: entry.name.clone(),
            kind: kind.clone(),
            activity,
            request,
            cancel,
            previous: installed_version,
            markers: entry.files_to_add.clone(),
        };
        self.set_install_state(&spec.key, Some(InstallState::Installing));
        self.send(AppAction::Activity(ActivityEvent::Started {
            id: activity,
            game_id: app,
            kind,
            title: spec.title.clone(),
            bytes_total: None,
        }));

        let host = self.clone();
        let started = thread::Builder::new().name(format!("reclaw-install-{app}")).spawn(move || host.run_job(installer, spec));
        if let Err(error) = started {
            installs.jobs().remove(&app);
            self.set_install_state(&key_of(&entry), None);
            self.send(AppAction::Activity(ActivityEvent::Failed {
                id: activity,
                reason: "A background thread did not start".to_string(),
                details: vec![error.to_string()],
            }));
        }
    }

    /// The version installed in the app's folder, from its recorded folder or the default one.
    fn installed_version(&self, entry: &AppEntry) -> Option<String> {
        let folder = paths::folder_of(entry, &self.default_location(), self.inner.installs.home.as_deref()).ok()?;
        reclaw_install::layout::installed_version(&folder)
    }

    fn run_job(&self, installer: reclaw_install::Installer, spec: Spec) {
        let end = self.work(&installer, &spec);
        self.inner.installs.jobs().remove(&spec.app);
        self.end_job(&spec, end);
    }

    fn work(&self, installer: &reclaw_install::Installer, spec: &Spec) -> End {
        let resolved: Resolved = match installer.resolve(&spec.request) {
            Ok(Plan::Ready(resolved)) => *resolved,
            Ok(Plan::Choose { release, choices }) => {
                // Several builds fit and nothing says which; take the one most likely to just work, and say so.
                let Some(best) = pick_best(&choices) else {
                    return End::Failed(InstallError::NoDownload("No download could be chosen for this release.".to_string()));
                };
                tracing::info!(app = spec.app, release = %release.tag, chosen = %best, among = choices.len(), "several downloads fit; taking the best ranked");
                let mut request = spec.request.clone();
                request.asset = Some(best);
                match installer.resolve(&request) {
                    Ok(Plan::Ready(resolved)) => *resolved,
                    Ok(Plan::Choose { .. }) => {
                        return End::Failed(InstallError::NoDownload("The download to take is still ambiguous.".to_string()));
                    }
                    Err(error) => return End::Failed(error),
                }
            }
            Err(error) => return End::Failed(error),
        };
        if resolved.already_installed {
            return End::Current(resolved.release.tag.clone());
        }
        let sink = |step: Step| {
            self.send(AppAction::Activity(ActivityEvent::Progress {
                id: spec.activity,
                stage: ui_stage(step.stage),
                bytes_done: step.done,
                bytes_total: step.total,
                rate: step.rate,
            }));
        };
        match installer.install(&spec.request, &resolved, &spec.cancel, &mut |step| sink(step)) {
            Ok(done) => End::Done(Box::new(done)),
            Err(error) if error.is_cancelled() => End::Cancelled,
            Err(error) => End::Failed(error),
        }
    }

    fn end_job(&self, spec: &Spec, end: End) {
        let id = spec.activity;
        match end {
            End::Done(done) => {
                tracing::info!(app = spec.app, version = %done.version, folder = %shown(&done.folder), "install finished");
                self.add_markers(&spec.title, &done.folder, &spec.markers, spec.previous.is_some());
                self.set_install_state(&spec.key, Some(InstallState::Installed { version: done.version.clone(), latest: None }));
                let changelog = (spec.kind == Kind::Update).then(|| Changelog {
                    from: spec.previous.clone().unwrap_or_default(),
                    to: done.version.clone(),
                    notes: done.notes.clone(),
                    url: (!done.page.is_empty()).then(|| done.page.clone()),
                });
                self.send(AppAction::Activity(ActivityEvent::Finished { id, changelog }));
                // A game that takes mods now has a folder to put them in.
                self.refresh_mods(false);
            }
            End::Current(version) => {
                self.set_install_state(&spec.key, Some(InstallState::Installed { version: version.clone(), latest: None }));
                self.send(AppAction::Activity(ActivityEvent::Cancelled { id }));
                self.tell(Notice::note(
                    &format!("{} is up to date", spec.title),
                    &format!("Version {version} is already installed"),
                    vec![],
                ));
            }
            End::Cancelled => {
                self.restore_state(spec);
                self.send(AppAction::Activity(ActivityEvent::Cancelled { id }));
            }
            End::Failed(error) => {
                tracing::warn!(app = spec.app, title = %spec.title, %error, "the install failed");
                match &spec.previous {
                    Some(_) => self.restore_state(spec),
                    None => self.set_install_state(&spec.key, Some(InstallState::Failed)),
                }
                let mut details = vec![error.to_string()];
                details.extend(error.hint());
                details.push(format!(
                    "The log has more: {}",
                    self.inner.logs_dir.as_ref().map_or_else(|| "reclaw.log".to_string(), |d| shown(&d.join("reclaw.log")))
                ));
                self.send(AppAction::Activity(ActivityEvent::Failed { id, reason: short(&error), details }));
            }
        }
    }

    /// Back to what was installed before the job began.
    fn restore_state(&self, spec: &Spec) {
        let state = spec.previous.clone().map(|version| InstallState::Installed { version, latest: None });
        self.set_install_state(&spec.key, state);
    }

    /// Put an app in the library (if it is not) and record the folder it is going into.
    fn keep_in_library_at(&self, app: u32, folder: &Path) -> bool {
        if self.add_to_library(app).is_none() {
            return false;
        }
        let mut state = self.state();
        let Some((true, entry)) = find(&state, app) else { return false };
        let text = shown(folder);
        if entry.install_path.as_deref() == Some(text.as_str()) {
            return true;
        }
        if !state.library_writable {
            drop(state);
            self.tell(read_only());
            return false;
        }
        let mut next = state.library.clone();
        for item in next.iter_mut().filter(|a| a.same_instance(&entry)) {
            item.install_path = Some(text.clone());
        }
        match self.inner.store.save(&next) {
            Ok(()) => {
                state.library = next;
                true
            }
            Err(error) => {
                drop(state);
                self.tell(save_failed(&error));
                false
            }
        }
    }
}

/// One line for the activity row.
fn short(error: &InstallError) -> String {
    let text = error.to_string();
    let first = text.lines().next().unwrap_or_default();
    if first.chars().count() > 120 { format!("{}...", first.chars().take(117).collect::<String>()) } else { first.to_string() }
}

/// Of several files that fit, the one most likely to just work: a plain archive before an AppImage (which needs FUSE), a
/// smaller archive format before a rarer one.
fn pick_best(choices: &[reclaw_install::Asset]) -> Option<String> {
    use reclaw_install::Format;
    let rank = |name: &str| match Format::of_name(name) {
        Format::TarGz => 0,
        Format::TarXz => 1,
        Format::Zip => 2,
        Format::SevenZip => 3,
        Format::AppImage => 4,
        Format::Bare => 5,
        Format::Rar => 6,
        Format::Exe => 7,
        Format::Unsupported(_) => 9,
    };
    choices.iter().min_by_key(|a| rank(&a.name)).map(|a| a.name.clone())
}

#[cfg(test)]
mod tests {
    use reclaw_install::Asset;

    use super::*;

    fn asset(name: &str) -> Asset {
        Asset { name: name.into(), url: format!("https://x.test/{name}"), size: None, sha256: None }
    }

    #[test]
    fn of_several_builds_the_plain_archive_wins() {
        assert_eq!(pick_best(&[asset("g.AppImage"), asset("g.zip"), asset("g.tar.gz")]).as_deref(), Some("g.tar.gz"));
        assert_eq!(pick_best(&[asset("g.AppImage"), asset("g.7z")]).as_deref(), Some("g.7z"));
        assert_eq!(pick_best(&[asset("g.AppImage"), asset("g")]).as_deref(), Some("g.AppImage"));
        assert_eq!(pick_best(&[]), None);
    }

    #[test]
    fn a_long_error_is_cut_to_one_line() {
        let long = InstallError::BadAnswer(format!("{}\nsecond line", "x".repeat(300)));
        assert!(short(&long).ends_with("...") && short(&long).chars().count() == 120 && !short(&long).contains('\n'));
        assert_eq!(short(&InstallError::NoProgram), "the download did not contain a program Reclaw can start");
    }
}
