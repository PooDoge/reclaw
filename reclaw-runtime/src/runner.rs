//! Running a Windows program on Linux: through Proton (a copy installed by Steam or a custom compatibility-tools folder), through
//! Wine, or through a command the person wrote. A port of Quiver's `WindowsRunnerService` with the same order in `Auto` (the
//! custom command, then Proton, then Wine) and the same variables, with one change: Proton installs are ordered by their version
//! numbers, not by their names as text (as text, `Proton 9` sorts after `Proton 10`).
//!
//! Everything that looks at the machine takes a [`Probe`], so the rules are tested with folders made for the test.
use std::{
    cmp::Ordering,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RunnerKind {
    #[default]
    Auto,
    Wine,
    Proton,
    Custom,
}

impl RunnerKind {
    /// The word stored in the library (`linuxRunner`); anything unknown is `Auto`.
    pub fn parse(text: Option<&str>) -> Self {
        match text.map(|t| t.trim().to_ascii_lowercase()).as_deref() {
            Some("wine") => Self::Wine,
            Some("proton") => Self::Proton,
            Some("custom") => Self::Custom,
            _ => Self::Auto,
        }
    }
}

/// What the library says about one app's runner, plus the person's global command.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct RunnerConfig {
    pub kind: RunnerKind,
    pub prefix: Option<PathBuf>,
    pub proton: Option<PathBuf>,
    /// This app's own command (`linuxCustomLaunchCommand`).
    pub custom: Option<String>,
    /// The command for every app (`LinuxWindowsLaunchCommand` in Quiver's settings).
    pub global_custom: Option<String>,
}

/// Where to look for runners: the search path and the home folder (Steam lives in it).
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Probe {
    pub path: Vec<PathBuf>,
    pub home: PathBuf,
}

impl Probe {
    pub fn from_env(home: PathBuf) -> Self {
        Self { path: std::env::var_os("PATH").map(|p| std::env::split_paths(&p).collect()).unwrap_or_default(), home }
    }

    fn command(&self, name: &str) -> Option<PathBuf> {
        self.path.iter().map(|dir| dir.join(name)).find(|candidate| is_executable(candidate))
    }

    fn steam_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for relative in [".steam/root", ".steam/steam", ".local/share/Steam", ".var/app/com.valvesoftware.Steam/.local/share/Steam"] {
            let root = self.home.join(relative);
            if root.is_dir() && !roots.contains(&root) {
                roots.push(root);
            }
        }
        roots
    }
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ProtonInstall {
    pub name: String,
    pub executable: PathBuf,
    pub steam_root: PathBuf,
}

/// Text compared the way a person reads version numbers: runs of digits by value, the rest by letters.
fn natural(a: &str, b: &str) -> Ordering {
    let chunks = |s: &str| {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, text)) if *d == digit => text.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    };
    let (x, y) = (chunks(&a.to_lowercase()), chunks(&b.to_lowercase()));
    for (p, q) in x.iter().zip(&y) {
        let order = match (p.0, q.0) {
            (true, true) => {
                p.1.trim_start_matches('0')
                    .len()
                    .cmp(&q.1.trim_start_matches('0').len())
                    .then_with(|| p.1.trim_start_matches('0').cmp(q.1.trim_start_matches('0')))
            }
            _ => p.1.cmp(&q.1),
        };
        if order != Ordering::Equal {
            return order;
        }
    }
    x.len().cmp(&y.len())
}

/// Every Proton found: Steam's own (`steamapps/common/Proton*`) and custom ones (`compatibilitytools.d/*Proton*`), newest first.
pub fn proton_installs(probe: &Probe) -> Vec<ProtonInstall> {
    let mut found = Vec::new();
    for root in probe.steam_roots() {
        for (parent, needle) in [(root.join("steamapps/common"), "proton"), (root.join("compatibilitytools.d"), "proton")] {
            let Ok(entries) = fs::read_dir(&parent) else { continue };
            let mut dirs: Vec<(String, PathBuf)> = entries
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| (e.file_name().to_string_lossy().into_owned(), e.path()))
                .filter(|(name, _)| name.to_ascii_lowercase().contains(needle))
                .collect();
            // Newest numbered release first; "Experimental" (a build that changes under you) last, only when nothing else is there.
            dirs.sort_by(|a, b| {
                a.0.to_lowercase()
                    .contains("experimental")
                    .cmp(&b.0.to_lowercase().contains("experimental"))
                    .then_with(|| natural(&b.0, &a.0))
            });
            for (name, dir) in dirs {
                let executable = dir.join("proton");
                if executable.is_file() && !found.iter().any(|f: &ProtonInstall| f.executable == executable) {
                    found.push(ProtonInstall { name, executable, steam_root: root.clone() });
                }
            }
        }
    }
    found
}

/// Where Steam's root is for a Proton at `executable` (the nearest folder up the tree that holds `steamapps` or `steam.sh`).
fn steam_root_of(executable: &Path) -> Option<PathBuf> {
    executable.ancestors().skip(1).find(|dir| dir.join("steamapps").is_dir() || dir.join("steam.sh").is_file()).map(Path::to_path_buf)
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RunnerCommand {
    pub program: PathBuf,
    pub args: Vec<OsString>,
    pub env: Vec<(OsString, OsString)>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RunnerError {
    #[error("no Windows runner is available: install Proton through Steam, or Wine, or write a command in the app's launch options")]
    None,
    #[error("the {0} runner was asked for, but it is not installed")]
    Missing(&'static str),
    #[error("the runner command is not usable: {0}")]
    BadCommand(String),
    #[error("a folder for the runner could not be made: {path}: {error}")]
    Prefix { path: PathBuf, error: String },
}

/// Words of a command line, with `'single'` and `"double"` quotes and `\` escapes; an unmatched quote is an error.
pub fn split_command(command: &str) -> Result<Vec<String>, String> {
    let (mut tokens, mut current, mut single, mut double, mut escaping, mut started) =
        (Vec::new(), String::new(), false, false, false, false);
    for c in command.chars() {
        if escaping {
            current.push(c);
            escaping = false;
        } else if c == '\\' && !single {
            escaping = true;
            started = true;
        } else if c == '"' && !single {
            double = !double;
            started = true;
        } else if c == '\'' && !double {
            single = !single;
            started = true;
        } else if c.is_whitespace() && !single && !double {
            if started {
                tokens.push(std::mem::take(&mut current));
                started = false;
            }
        } else {
            current.push(c);
            started = true;
        }
    }
    if escaping || single || double {
        return Err("an unmatched quote or a trailing backslash".to_string());
    }
    if started {
        tokens.push(current);
    }
    Ok(tokens)
}

/// A custom command with `{exe}`, `{gamePath}` and `{exeDir}` filled in; the program is added at the end if none is named.
fn custom_command(template: &str, exe: &Path, game: &Path) -> Result<RunnerCommand, RunnerError> {
    let mut text = template.trim().to_string();
    if !["{exe}", "{gamePath}", "{exeDir}"].iter().any(|p| text.contains(p)) {
        text.push_str(" {exe}");
    }
    let exe_dir = exe.parent().unwrap_or(game);
    let words = split_command(&text).map_err(RunnerError::BadCommand)?;
    let words: Vec<String> = words
        .into_iter()
        .map(|w| {
            w.replace("{exe}", &exe.to_string_lossy())
                .replace("{gamePath}", &game.to_string_lossy())
                .replace("{exeDir}", &exe_dir.to_string_lossy())
        })
        .collect();
    let Some((program, args)) = words.split_first().filter(|(p, _)| !p.trim().is_empty()) else {
        return Err(RunnerError::BadCommand("it is empty".to_string()));
    };
    Ok(RunnerCommand { program: program.into(), args: args.iter().map(OsString::from).collect(), env: Vec::new() })
}

fn make_dir(path: &Path) -> Result<(), RunnerError> {
    fs::create_dir_all(path).map_err(|e| RunnerError::Prefix { path: path.to_path_buf(), error: e.to_string() })
}

fn wine_command(probe: &Probe, exe: &Path, game: &Path, prefix: Option<&Path>) -> Result<Option<RunnerCommand>, RunnerError> {
    let Some(binary) = probe.command("wine64").or_else(|| probe.command("wine")) else { return Ok(None) };
    let prefix = prefix.map_or_else(|| game.join(".wine-prefix"), Path::to_path_buf);
    make_dir(&prefix)?;
    Ok(Some(RunnerCommand { program: binary, args: vec![exe.into()], env: vec![("WINEPREFIX".into(), prefix.into_os_string())] }))
}

/// A number that is the same for the same program, so Proton keeps one prefix per program: FNV-1a, 31 bits.
fn compat_app_id(exe: &Path) -> String {
    let hash = exe.to_string_lossy().bytes().fold(2_166_136_261u32, |h, b| (h ^ u32::from(b)).wrapping_mul(16_777_619));
    (hash & 0x7fff_ffff).to_string()
}

fn proton_command(
    probe: &Probe,
    exe: &Path,
    game: &Path,
    prefix: Option<&Path>,
    chosen: Option<&Path>,
) -> Result<Option<RunnerCommand>, RunnerError> {
    let install = chosen
        .filter(|p| p.is_file())
        .and_then(|p| {
            steam_root_of(p).or_else(|| probe.steam_roots().into_iter().next()).map(|root| ProtonInstall {
                name: String::new(),
                executable: p.to_path_buf(),
                steam_root: root,
            })
        })
        .or_else(|| proton_installs(probe).into_iter().next());
    let Some(install) = install else { return Ok(None) };
    let compat = prefix.map_or_else(|| game.join(".steam-compat-data"), Path::to_path_buf);
    make_dir(&compat)?;
    let id = compat_app_id(exe);
    Ok(Some(RunnerCommand {
        program: install.executable,
        args: vec!["waitforexitandrun".into(), exe.into()],
        env: vec![
            ("STEAM_COMPAT_CLIENT_INSTALL_PATH".into(), install.steam_root.into_os_string()),
            ("STEAM_COMPAT_DATA_PATH".into(), compat.into_os_string()),
            // Not in Quiver's list: Proton's own script documents it as where the program lives (its container mounts it).
            ("STEAM_COMPAT_INSTALL_PATH".into(), game.as_os_str().to_owned()),
            ("STEAM_COMPAT_APP_ID".into(), id.clone().into()),
            ("SteamAppId".into(), id.clone().into()),
            ("SteamGameId".into(), id.into()),
        ],
    }))
}

/// Whether some runner could run a program with this configuration.
pub fn available(config: &RunnerConfig, probe: &Probe) -> bool {
    let has_wine = probe.command("wine64").or_else(|| probe.command("wine")).is_some();
    let has_proton = || !proton_installs(probe).is_empty() || config.proton.as_deref().is_some_and(Path::is_file);
    let any_custom = config.custom.as_deref().is_some_and(|c| !c.trim().is_empty())
        || config.global_custom.as_deref().is_some_and(|c| !c.trim().is_empty());
    match config.kind {
        RunnerKind::Custom => any_custom,
        RunnerKind::Wine => has_wine,
        RunnerKind::Proton => has_proton(),
        RunnerKind::Auto => config.global_custom.as_deref().is_some_and(|c| !c.trim().is_empty()) || has_proton() || has_wine,
    }
}

/// The command that runs `exe` (inside `game`, the app's folder).
pub fn command(config: &RunnerConfig, probe: &Probe, exe: &Path, game: &Path) -> Result<RunnerCommand, RunnerError> {
    let nonblank = |t: &Option<String>| t.as_deref().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string);
    let (own, global) = (nonblank(&config.custom), nonblank(&config.global_custom));
    match config.kind {
        RunnerKind::Custom => custom_command(&own.or(global).ok_or(RunnerError::None)?, exe, game),
        RunnerKind::Wine => wine_command(probe, exe, game, config.prefix.as_deref())?.ok_or(RunnerError::Missing("Wine")),
        RunnerKind::Proton => {
            proton_command(probe, exe, game, config.prefix.as_deref(), config.proton.as_deref())?.ok_or(RunnerError::Missing("Proton"))
        }
        RunnerKind::Auto => {
            if let Some(template) = global {
                return custom_command(&template, exe, game);
            }
            if let Some(found) = proton_command(probe, exe, game, config.prefix.as_deref(), config.proton.as_deref())? {
                return Ok(found);
            }
            wine_command(probe, exe, game, config.prefix.as_deref())?.ok_or(RunnerError::None)
        }
    }
}

#[cfg(test)]
mod tests;
