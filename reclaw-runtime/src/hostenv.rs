//! The environment a game starts with. The launcher may itself run from an AppImage or under Steam, which leaves variables that
//! break the programs it starts (a library path pointing into the launcher's own bundle, Steam's overlay preloaded into a game
//! that is not a Steam game). Quiver strips the same ones; so does this, once, in one place.
use std::ffi::OsString;

use crate::state::LaunchSpec;

/// Variables a child must never inherit.
pub const HOST_BREAKING: [&str; 9] =
    ["LD_LIBRARY_PATH", "LD_PRELOAD", "QT_PLUGIN_PATH", "QTDIR", "QT_QPA_PLATFORM_PLUGIN_PATH", "APPDIR", "APPIMAGE", "ARGV0", "OWD"];

const DEFAULT_PATH: &str = "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin";

/// Is this `PATH` entry inside an AppImage's mount (or the directory `APPDIR` names)?
fn in_appimage(entry: &str, appdir: Option<&str>) -> bool {
    !entry.trim().is_empty() && (appdir.is_some_and(|dir| !dir.is_empty() && entry.starts_with(dir)) || entry.contains("/.mount_"))
}

/// `PATH` without the AppImage's own folders; the system's default when nothing is left.
pub fn clean_path(path: &str, appdir: Option<&str>) -> String {
    let kept: Vec<&str> = path.split(':').filter(|entry| !entry.is_empty() && !in_appimage(entry, appdir)).collect();
    if kept.is_empty() { DEFAULT_PATH.to_string() } else { kept.join(":") }
}

/// Make `spec` start with a clean environment, given the launcher's own (`inherited`). Variables the spec already sets are left
/// alone, so a runner's `WINEPREFIX` is not stripped, and a variable set later overrides.
pub fn apply(spec: &mut LaunchSpec, inherited: impl IntoIterator<Item = (OsString, OsString)>) {
    let inherited: Vec<(OsString, OsString)> = inherited.into_iter().collect();
    let appdir = inherited.iter().find(|(k, _)| k == "APPDIR").and_then(|(_, v)| v.to_str()).map(str::to_string);
    for (key, _) in &inherited {
        let name = key.to_string_lossy();
        let strip = HOST_BREAKING.contains(&name.as_ref()) || name.to_ascii_uppercase().starts_with("APPIMAGE_");
        let set_by_spec = spec.env.iter().any(|(k, _)| k == key);
        if strip && !set_by_spec {
            spec.env_remove.push(key.clone());
        }
    }
    if !spec.env.iter().any(|(k, _)| k == "PATH")
        && let Some((_, path)) = inherited.iter().find(|(k, _)| k == "PATH")
        && let Some(path) = path.to_str()
    {
        let cleaned = clean_path(path, appdir.as_deref());
        if cleaned != path {
            spec.env.push(("PATH".into(), cleaned.into()));
        }
    }
    // A program started with a working directory should see it as `$PWD` too.
    if let Some(cwd) = &spec.cwd
        && !spec.env.iter().any(|(k, _)| k == "PWD")
    {
        spec.env.push(("PWD".into(), cwd.as_os_str().to_owned()));
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;

    use super::*;

    fn value_of<'a>(spec: &'a LaunchSpec, name: &str) -> Option<&'a OsStr> {
        spec.env.iter().rev().find(|(k, _)| k == name).map(|(_, v)| v.as_os_str())
    }

    fn env(pairs: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        pairs.iter().map(|(k, v)| ((*k).into(), (*v).into())).collect()
    }

    #[test]
    fn variables_that_break_a_child_are_removed() {
        let mut spec = LaunchSpec::new("/game");
        apply(
            &mut spec,
            env(&[("LD_PRELOAD", "gameoverlay.so"), ("LD_LIBRARY_PATH", "/tmp/.mount_x/lib"), ("HOME", "/home/me"), ("APPIMAGE_FOO", "1")]),
        );
        let removed: Vec<_> = spec.env_remove.iter().map(|k| k.to_string_lossy().into_owned()).collect();
        assert!(
            removed.contains(&"LD_PRELOAD".to_string())
                && removed.contains(&"LD_LIBRARY_PATH".to_string())
                && removed.contains(&"APPIMAGE_FOO".to_string()),
            "{removed:?}"
        );
        assert!(!removed.contains(&"HOME".to_string()));
    }

    #[test]
    fn a_runner_may_set_what_would_otherwise_be_stripped() {
        let mut spec = LaunchSpec::new("proton").env("LD_LIBRARY_PATH", "/steam/runtime/lib");
        apply(&mut spec, env(&[("LD_LIBRARY_PATH", "/tmp/.mount_x/lib")]));
        assert!(spec.env_remove.is_empty(), "the runner's own value is kept");
    }

    #[test]
    fn the_path_loses_the_appimages_folders_and_nothing_else() {
        assert_eq!(clean_path("/tmp/.mount_abc/usr/bin:/usr/bin:/home/me/bin", None), "/usr/bin:/home/me/bin");
        assert_eq!(clean_path("/opt/app/bin:/usr/bin", Some("/opt/app")), "/usr/bin");
        assert_eq!(clean_path("/tmp/.mount_abc/usr/bin", None), DEFAULT_PATH, "never an empty path");
        assert_eq!(clean_path("/usr/bin::/bin", None), "/usr/bin:/bin", "empty entries go");
    }

    #[test]
    fn the_path_is_only_set_when_it_changed() {
        let mut spec = LaunchSpec::new("/game");
        apply(&mut spec, env(&[("PATH", "/usr/bin:/bin")]));
        assert!(value_of(&spec, "PATH").is_none());
        let mut spec = LaunchSpec::new("/game");
        apply(&mut spec, env(&[("PATH", "/tmp/.mount_x/bin:/usr/bin")]));
        assert_eq!(value_of(&spec, "PATH"), Some(OsStr::new("/usr/bin")));
    }

    #[test]
    fn the_working_directory_is_also_pwd() {
        let mut spec = LaunchSpec::new("/game");
        spec.cwd = Some("/games/One".into());
        apply(&mut spec, env(&[]));
        assert_eq!(value_of(&spec, "PWD"), Some(OsStr::new("/games/One")));
    }
}
