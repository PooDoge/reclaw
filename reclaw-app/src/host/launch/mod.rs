//! Starting and stopping apps. The work of a process (own process group, a log file, a graceful stop that escalates) is the
//! supervisor's (`reclaw-runtime`); this decides *what* to start: the program in the app's folder, through a Windows runner when
//! it is a Windows program on Linux, with the person's launch settings and a clean environment.
//!
//! * this file: the launch and stop; `events`: what the supervisor reports, carried to the screens
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use reclaw_catalog::AppEntry;
use reclaw_install::{Platform, programs};
use reclaw_runtime::{LaunchError, LaunchSpec, Probe, RunState, RunnerConfig, RunnerKind, Supervisor};
use reclaw_ui::{launch_request::LaunchRequest, notices::Notice, store::AppAction};

use super::{Host, find, install::paths};

mod events;

/// What the host needs to start apps.
pub struct Runs {
    pub(super) supervisor: Supervisor,
    probe: Probe,
    /// Where games keep settings (`~/.config`) and data (`~/.local/share`), for the config edits a launch plan makes.
    config_base: PathBuf,
    data_base: PathBuf,
}

impl Runs {
    pub fn new(supervisor: Supervisor, home: Option<&Path>) -> Self {
        let home = home.map(Path::to_path_buf).unwrap_or_default();
        let env_dir = |name: &str, fallback: &str| {
            std::env::var_os(name).filter(|v| !v.is_empty()).map_or_else(|| home.join(fallback), PathBuf::from)
        };
        Self {
            supervisor,
            probe: Probe::from_env(home.clone()),
            config_base: env_dir("XDG_CONFIG_HOME", ".config"),
            data_base: env_dir("XDG_DATA_HOME", ".local/share"),
        }
    }

    /// For a test: look for runners only where it says.
    pub fn with_probe(mut self, probe: Probe) -> Self {
        self.probe = probe;
        self
    }
}

/// How long a graceful stop may take before the app is killed.
pub const STOP_GRACE: Duration = Duration::from_secs(8);

/// The program to start in an app's folder: the one the person chose (Quiver's `selected_executable.txt`), else the nearest to the
/// top. A top-level search comes first so that a launcher script beside the game beats a helper deep inside it.
fn choose_program(folder: &Path, platform: Platform) -> Option<PathBuf> {
    if let Ok(text) = fs::read_to_string(folder.join("selected_executable.txt")) {
        let chosen = PathBuf::from(text.trim());
        let inside = reclaw_install::layout::normalize(&chosen).starts_with(reclaw_install::layout::normalize(folder));
        if inside && chosen.is_file() {
            return Some(chosen);
        }
    }
    let top = programs::find(folder, false, platform);
    let found = if top.programs.is_empty() { programs::find(folder, true, platform) } else { top };
    found.programs.into_iter().next()
}

fn runner_config(entry: &AppEntry) -> RunnerConfig {
    let path = |text: &Option<String>| text.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(PathBuf::from);
    RunnerConfig {
        kind: RunnerKind::parse(entry.linux_runner.as_deref()),
        prefix: path(&entry.linux_prefix_path),
        proton: path(&entry.linux_proton_path),
        custom: entry.linux_custom_launch_command.clone(),
        global_custom: None,
    }
}

impl Host {
    /// The file an app's output goes to: `<logs>/games/<folder name>.log`, started afresh when it has grown large.
    pub(super) fn game_log(&self, app: u32) -> Option<PathBuf> {
        let (_, entry) = find(&self.state(), app)?;
        let name = paths::folder_name(&entry).ok()?;
        Some(self.inner.logs_dir.as_ref()?.join("games").join(format!("{name}.log")))
    }

    fn prepare_log(&self, app: u32) -> Option<PathBuf> {
        let path = self.game_log(app)?;
        if let Some(dir) = path.parent()
            && let Err(error) = fs::create_dir_all(dir)
        {
            tracing::warn!(dir = %dir.display(), %error, "the folder for apps' output could not be made; it will be lost");
            return None;
        }
        // Several sessions share one file, so it is cut when it gets big rather than growing for ever.
        if fs::metadata(&path).is_ok_and(|m| m.len() > 5 * 1024 * 1024) {
            let _ = fs::remove_file(&path);
        }
        Some(path)
    }

    /// Start an app with the person's launch settings.
    pub fn launch(&self, app: u32, request: &LaunchRequest) {
        let runs = &self.inner.runs;
        if runs.supervisor.state(app).is_active() {
            tracing::debug!(app, "Play was pressed for an app that is already running");
            return;
        }
        let Some((_, entry)) = find(&self.state(), app) else { return };
        let platform = self.inner.installs.platform;
        let folder = match paths::folder_of(&entry, &self.default_location(), self.inner.installs.home.as_deref()) {
            Ok(folder) => folder,
            Err(problem) => {
                self.tell(Notice::problem(
                    &format!("{} cannot be started", entry.name),
                    &problem.message(),
                    vec![problem.hint().to_string()],
                ));
                return;
            }
        };
        if !folder.is_dir() {
            self.tell(Notice::problem(
                &format!("{} cannot be started", entry.name),
                "Its folder was not found",
                vec![format!("Reclaw looked in {}", folder.display()), "It may have been moved or deleted. Install it again.".to_string()],
            ));
            return;
        }
        let Some(program) = choose_program(&folder, platform) else {
            self.tell(Notice::problem(
                &format!("{} has nothing to start", entry.name),
                "No program was found in its folder",
                vec![
                    format!("Folder: {}", folder.display()),
                    "The install may be incomplete. Use Verify, or install it again.".to_string(),
                ],
            ));
            return;
        };
        let spec = match self.build_spec(app, &entry, &folder, &program, request) {
            Ok(spec) => spec,
            Err(notice) => {
                self.tell(notice);
                return;
            }
        };
        if let Some(problem) = &request.problem {
            self.tell(Notice::note("Launch options were not used", problem, vec![]));
        }
        self.apply_plan_config(&entry, &folder, request);

        tracing::info!(
            app,
            program = %spec.program.display(),
            args = ?spec.args,
            cwd = ?spec.cwd,
            removed_env = ?spec.env_remove,
            log = ?spec.log,
            "launching"
        );
        self.send(AppAction::SetRun { id: app, run: RunState::Starting });
        if let Err(error) = runs.supervisor.start(app, spec) {
            tracing::warn!(app, %error, "an app could not be started");
            self.send(AppAction::SetRun { id: app, run: RunState::Idle });
            self.tell(launch_failed(&entry.name, &error, &program));
        }
    }

    fn build_spec(&self, app: u32, entry: &AppEntry, folder: &Path, program: &Path, request: &LaunchRequest) -> Result<LaunchSpec, Notice> {
        let runs = &self.inner.runs;
        let platform = self.inner.installs.platform;
        let extra: Vec<OsString> = request.plan.args.iter().chain(&request.options).map(OsString::from).collect();
        let is_exe = program.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));

        let mut spec = if platform.is_linux() && is_exe {
            let config = runner_config(entry);
            if !reclaw_runtime::runner_available(&config, &runs.probe) {
                return Err(Notice::problem(
                    &format!("{} needs Wine or Proton", entry.name),
                    "This is a Windows program and no way to run one was found",
                    vec![
                        "Steam's Proton is found automatically once Steam has installed one (any game that uses it will do).".to_string(),
                        "Or install Wine from your system's package manager.".to_string(),
                        "A command of your own can go in the app's linuxCustomLaunchCommand in apps.json.".to_string(),
                    ],
                ));
            }
            let command = reclaw_runtime::runner_command(&config, &runs.probe, program, folder).map_err(|e| {
                Notice::problem(
                    &format!("{} cannot be started", entry.name),
                    &e.to_string(),
                    vec![format!("Program: {}", program.display())],
                )
            })?;
            let mut spec = LaunchSpec::new(command.program);
            spec.args = command.args;
            spec.env = command.env;
            spec.cwd = Some(folder.to_path_buf());
            spec
        } else {
            if let Err(error) = programs::make_file_runnable(program) {
                tracing::warn!(program = %program.display(), %error, "a program could not be made executable");
            }
            let mut spec = LaunchSpec::new(program);
            spec.cwd = Some(program.parent().unwrap_or(folder).to_path_buf());
            spec
        };
        spec.args.extend(extra);
        spec.env.extend(request.plan.env.iter().map(|(k, v)| (OsString::from(k), OsString::from(v))));
        spec.log = self.prepare_log(app);
        reclaw_runtime::clean_environment(&mut spec, std::env::vars_os());
        Ok(spec)
    }

    /// Edits to the app's own config files that the launch settings ask for. A failure is told and the app still starts: the person
    /// should know their choice was not applied, but not be kept from playing.
    fn apply_plan_config(&self, entry: &AppEntry, folder: &Path, request: &LaunchRequest) {
        if request.plan.config_edits.is_empty() {
            return;
        }
        let runs = &self.inner.runs;
        let bases =
            reclaw_games::settings::Bases { install: folder.to_path_buf(), config: runs.config_base.clone(), data: runs.data_base.clone() };
        if let Err(error) = request.plan.apply_config(&bases) {
            tracing::warn!(app = %entry.name, %error, "launch settings could not be written to the app's config");
            self.tell(Notice::problem(
                &format!("A setting was not applied to {}", entry.name),
                "It starts with the setting it had",
                vec![error.to_string()],
            ));
        }
    }

    /// Ask a running app to quit; a second press kills it.
    pub(crate) fn stop(&self, app: u32) {
        let supervisor = &self.inner.runs.supervisor;
        match supervisor.stop(app) {
            Ok(()) => self.send(AppAction::SetRun { id: app, run: supervisor.state(app) }),
            Err(error) => tracing::debug!(app, %error, "Stop was pressed for an app that is not running"),
        }
    }
}

fn launch_failed(title: &str, error: &LaunchError, program: &Path) -> Notice {
    let mut details = vec![error.to_string(), format!("Program: {}", program.display())];
    if let LaunchError::Spawn { source, .. } = error {
        match source.kind() {
            std::io::ErrorKind::PermissionDenied => {
                details.push("The file is not executable, or the disk it is on is mounted noexec.".to_string())
            }
            std::io::ErrorKind::NotFound => details
                .push("The file, or the interpreter it names, is missing. A program built for another system looks like this.".to_string()),
            _ => {}
        }
    }
    Notice::problem(&format!("{title} could not be started"), &error.to_string(), details)
}

#[cfg(test)]
mod tests;
