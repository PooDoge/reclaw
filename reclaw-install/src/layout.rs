//! The shape of an app folder: Reclaw's own marker files, flattening the folder an archive wraps everything in, and moving a
//! freshly unpacked tree over the installed one.
use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use crate::{error::InstallError, platform::Platform, programs};

/// Written last: the folder holds a complete install of this version.
pub const VERSION_FILE: &str = "version.txt";
/// Written first when there is nothing installed yet, removed last: a folder with this is an install that did not finish.
pub const INCOMPLETE_FILE: &str = "install-incomplete.txt";
/// Where an archive is unpacked before it is moved into place. Hidden, inside the app folder so a move never crosses disks.
pub const STAGE_DIR: &str = ".reclaw-stage";

/// Files that belong to the launcher, not the app: never the program, never a reason to keep a wrapper folder.
pub fn is_metadata_file(name: &str) -> bool {
    ["version.txt", INCOMPLETE_FILE, "LastPlayed.txt", "selected_executable.txt", "flatpak-install.json", "flatpak-install.pending.json"]
        .iter()
        .any(|m| name.eq_ignore_ascii_case(m))
}

/// A path with `.` and `..` resolved by reading it, without touching the disk.
pub fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                if !out.pop() {
                    out.push("..");
                }
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn io_error(what: &str, path: &Path, error: &io::Error) -> InstallError {
    InstallError::io(format!("{what} {}", path.display()), error)
}

/// The files and folders directly inside `dir`.
fn children(dir: &Path) -> Result<Vec<PathBuf>, InstallError> {
    fs::read_dir(dir)
        .map_err(|e| io_error("reading", dir, &e))?
        .map(|entry| entry.map(|e| e.path()).map_err(|e| io_error("reading", dir, &e)))
        .collect()
}

fn is_real_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|m| m.is_dir())
}

/// If everything in `root` is one folder, bring that folder's contents up a level, until a program is at the top (or the
/// tree is not just a wrapper). Sibling files or folders are part of the app and keep the structure as it is.
pub fn hoist_wrapper(root: &Path, platform: Platform) -> Result<(), InstallError> {
    for _ in 0..16 {
        if !programs::find(root, false, platform).programs.is_empty() {
            return Ok(());
        }
        let entries = children(root)?;
        let (dirs, files): (Vec<_>, Vec<_>) = entries.into_iter().partition(|p| is_real_dir(p));
        let files: Vec<_> = files.into_iter().filter(|f| !f.file_name().and_then(|n| n.to_str()).is_some_and(is_metadata_file)).collect();
        let [wrapper] = dirs.as_slice() else { return Ok(()) };
        if !files.is_empty() || programs::find(wrapper, true, platform).programs.is_empty() {
            return Ok(());
        }
        // The wrapper may hold something with its own name; move it aside first so the contents can come up.
        let aside = root.join(format!("{}.reclaw-hoist", wrapper.file_name().and_then(|n| n.to_str()).unwrap_or("wrapper")));
        fs::rename(wrapper, &aside).map_err(|e| io_error("moving", wrapper, &e))?;
        merge_into(&aside, root)?;
        fs::remove_dir_all(&aside).map_err(|e| io_error("removing", &aside, &e))?;
        tracing::debug!(wrapper = %wrapper.display(), "moved the contents of the folder the archive wrapped them in up a level");
    }
    Ok(())
}

fn remove_any(path: &Path) -> Result<(), InstallError> {
    let meta = fs::symlink_metadata(path).map_err(|e| io_error("looking at", path, &e))?;
    let result = if meta.is_dir() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
    result.map_err(|e| io_error("replacing", path, &e))
}

/// Move everything in `from` into `to`, over what is there. Folders merge; a file replaces a file; nothing in `to` that `from`
/// lacks is touched, so settings and saves an app keeps beside its program survive an update.
pub fn merge_into(from: &Path, to: &Path) -> Result<(), InstallError> {
    fs::create_dir_all(to).map_err(|e| io_error("making", to, &e))?;
    for source in children(from)? {
        let Some(name) = source.file_name() else { continue };
        let target = to.join(name);
        let source_is_dir = is_real_dir(&source);
        match fs::symlink_metadata(&target) {
            Ok(existing) if source_is_dir && existing.is_dir() => {
                merge_into(&source, &target)?;
                continue;
            }
            Ok(_) => remove_any(&target)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error("looking at", &target, &error)),
        }
        fs::rename(&source, &target).map_err(|e| io_error("moving into place", &target, &e))?;
    }
    Ok(())
}

/// The version recorded in a folder, if it holds a finished install.
pub fn installed_version(folder: &Path) -> Option<String> {
    if folder.join(INCOMPLETE_FILE).exists() {
        return None;
    }
    let text = fs::read_to_string(folder.join(VERSION_FILE)).ok()?;
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_string())
}

/// Whether the folder holds a finished install of something that can be started.
pub fn is_complete(folder: &Path, platform: Platform) -> bool {
    folder.is_dir() && !folder.join(INCOMPLETE_FILE).exists() && !programs::find(folder, true, platform).programs.is_empty()
}

#[cfg(test)]
mod tests;
