use std::thread;

use reclaw_catalog::AppEntry;
use reclaw_install::{Host as RepoHost, Platform, layout, remove, select_release};
use reclaw_ui::{
    activity::ActivityId,
    catalog_data::{InstallState, key_of},
    notices::Notice,
};

use super::{paths, shown};
use crate::{
    browse,
    host::{Host, find},
};

impl Host {
    /// The app's folder, or a notice saying why there is none.
    fn folder_for(&self, app: u32) -> Option<(AppEntry, std::path::PathBuf)> {
        let (_, entry) = find(&self.state(), app)?;
        match paths::folder_of(&entry, &self.default_location(), self.inner.installs.home.as_deref()) {
            Ok(folder) => Some((entry, folder)),
            Err(problem) => {
                self.tell(Notice::problem(
                    &format!("{} has no usable folder", entry.name),
                    &problem.message(),
                    vec![problem.hint().to_string()],
                ));
                None
            }
        }
    }

    pub(crate) fn cancel_activity(&self, id: ActivityId) {
        if self.inner.installs.cancel(id) {
            tracing::info!(activity = id, "an install was cancelled by the user");
        }
    }

    pub(crate) fn open_app_folder(&self, app: u32) {
        let Some((entry, folder)) = self.folder_for(app) else { return };
        if !folder.is_dir() {
            self.tell(Notice::problem(
                &format!("{} has no folder yet", entry.name),
                "It is not installed, or it was moved or deleted",
                vec![format!("Reclaw looked in {}", shown(&folder))],
            ));
            return;
        }
        if let Err(error) = browse::open_folder(&folder) {
            tracing::warn!(folder = %folder.display(), %error, "an app's folder could not be opened");
            self.tell(Notice::problem("The folder could not be opened", &error.to_string(), vec![format!("It is {}", shown(&folder))]));
        }
    }

    /// Delete an app's files. The app stays in the library.
    pub(crate) fn uninstall(&self, app: u32) {
        if self.inner.installs.is_busy(app) {
            self.tell(Notice::note("Wait for it to finish", "An install or update is running for this app; cancel it first", vec![]));
            return;
        }
        let Some((entry, folder)) = self.folder_for(app) else { return };
        let (host, key, platform) = (self.clone(), key_of(&entry), self.inner.installs.platform);
        let protected = self.inner.installs.protected(&self.inner.protected);
        let work = move || match remove::uninstall(&folder, &protected, platform) {
            Ok(()) => {
                host.set_install_state(&key, None);
                host.forget_install_path(app);
                host.tell(Notice::note(
                    &format!("{} was uninstalled", entry.name),
                    &format!("Its files in {} were deleted", shown(&folder)),
                    vec![],
                ));
            }
            Err(error) => {
                tracing::warn!(app, folder = %folder.display(), %error, "an uninstall failed");
                let mut details = vec![error.to_string()];
                details.extend(error.hint());
                host.tell(Notice::problem(&format!("{} could not be uninstalled", entry.name), &error.to_string(), details));
            }
        };
        if let Err(error) = thread::Builder::new().name(format!("reclaw-uninstall-{app}")).spawn(work) {
            self.tell(Notice::problem("The uninstall could not start", "A background thread did not start", vec![error.to_string()]));
        }
    }

    /// Ask the host service what the newest release is and say whether it is newer than what is installed.
    pub(crate) fn check_update(&self, app: u32) {
        let Some(installer) = self.inner.installs.installer.clone() else {
            self.tell(Notice::problem("No network", "The network layer could not start, so updates cannot be checked", vec![]));
            return;
        };
        let Some((entry, folder)) = self.folder_for(app) else { return };
        let Some(installed) = layout::installed_version(&folder) else {
            self.tell(Notice::note(&format!("{} is not installed", entry.name), "There is nothing to check for updates", vec![]));
            return;
        };
        let (host, key) = (self.clone(), key_of(&entry));
        let work = move || {
            let repo_host = if entry.source == reclaw_catalog::RepoSource::Gitlab { RepoHost::GitLab } else { RepoHost::GitHub };
            let found = installer.releases().fetch(repo_host, entry.repository.trim(), entry.preferred_version.is_some(), true);
            match found {
                Ok(found) => {
                    let latest = select_release(&found.list, entry.preferred_version.as_deref(), found.latest_tag.as_deref(), false)
                        .map(|r| r.tag.clone());
                    match latest {
                        Some(tag) if reclaw_games::version::is_newer(&tag, &installed) => {
                            tracing::info!(app, installed = %installed, latest = %tag, "an update is available");
                            host.set_install_state(&key, Some(InstallState::Installed { version: installed, latest: Some(tag) }));
                        }
                        _ => {
                            host.set_install_state(&key, Some(InstallState::Installed { version: installed.clone(), latest: None }));
                            host.tell(Notice::note(
                                &format!("{} is up to date", entry.name),
                                &format!("Version {installed} is the newest"),
                                vec![],
                            ));
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(app, %error, "the update check failed");
                    let mut details = vec![error.to_string()];
                    details.extend(error.hint());
                    host.tell(Notice::problem(&format!("Could not check {} for updates", entry.name), &error.to_string(), details));
                }
            }
        };
        if let Err(error) = thread::Builder::new().name(format!("reclaw-check-{app}")).spawn(work) {
            self.tell(Notice::problem("The check could not start", "A background thread did not start", vec![error.to_string()]));
        }
    }

    /// Look at the folder again and say what is in it. (It does not compare files with the release: the release lists none.)
    pub(crate) fn verify(&self, app: u32) {
        let Some((entry, folder)) = self.folder_for(app) else { return };
        let platform = self.inner.installs.platform;
        let key = key_of(&entry);
        let version = layout::installed_version(&folder);
        let programs = reclaw_install::programs::find(&folder, true, platform);
        let mut details = vec![format!("Folder: {}", shown(&folder))];
        match (&version, programs.programs.first()) {
            (Some(version), Some(program)) => {
                details.push(format!("Version: {version}"));
                details.push(format!("Starts: {}", program.strip_prefix(&folder).unwrap_or(program).display()));
                if programs.needs_runner
                    || (platform != Platform::Windows && program.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe")))
                {
                    details.push("This is a Windows program; it needs Wine or Proton to run.".to_string());
                }
                self.set_install_state(&key, Some(InstallState::Installed { version: version.clone(), latest: None }));
                self.tell(Notice::note(&format!("{} looks complete", entry.name), &format!("Version {version} is installed"), details));
            }
            (Some(version), None) => {
                self.tell(Notice::problem(&format!("{} has nothing to start", entry.name), "The folder has a version but no program", {
                    details.push(format!("Version: {version}"));
                    details.push("Install it again to repair it.".to_string());
                    details
                }));
            }
            (None, _) => {
                self.set_install_state(&key, None);
                let why =
                    if folder.join(layout::INCOMPLETE_FILE).exists() { "An install did not finish" } else { "Nothing is installed there" };
                details.push("Install it again.".to_string());
                self.tell(Notice::problem(&format!("{} is not installed", entry.name), why, details));
            }
        }
    }

    /// Forget the folder a library entry was installed into (after an uninstall).
    pub(super) fn forget_install_path(&self, app: u32) {
        let mut state = self.state();
        let Some((true, entry)) = find(&state, app) else { return };
        if entry.install_path.is_none() || !state.library_writable {
            return;
        }
        let mut next = state.library.clone();
        for item in next.iter_mut().filter(|a| a.same_instance(&entry)) {
            item.install_path = None;
        }
        match self.inner.store.save(&next) {
            Ok(()) => state.library = next,
            Err(error) => {
                drop(state);
                tracing::warn!(app, %error, "the library could not be saved after an uninstall");
            }
        }
    }
}
