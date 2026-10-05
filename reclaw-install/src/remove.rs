//! Uninstalling: deleting an app's folder, and refusing to delete anything that is not clearly one. A folder name comes from a
//! settings text box and a file Reclaw wrote; a wrong one must never take a home folder with it.
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{error::InstallError, layout, platform::Platform};

/// Why a folder was not removed.
fn refuse(folder: &Path, why: &str) -> InstallError {
    tracing::warn!(folder = %folder.display(), why, "an uninstall was refused");
    InstallError::Unsupported(format!("{} was not removed: {why}.", folder.display()))
}

/// Check that `folder` is something Reclaw installed and nothing worth more than that lies at or above it.
/// `protected` is where the user's own files are (the home folder, Reclaw's own folders, the install location itself).
pub fn check_removable(folder: &Path, protected: &[PathBuf], platform: Platform) -> Result<(), InstallError> {
    if !folder.is_absolute() {
        return Err(refuse(folder, "its address is not a full path"));
    }
    if folder.components().count() < 4 {
        return Err(refuse(folder, "it is too close to the top of the disk"));
    }
    let clean = layout::normalize(folder);
    if clean != folder {
        return Err(refuse(folder, "its address is not in its simplest form"));
    }
    for guarded in protected {
        if guarded.starts_with(folder) {
            return Err(refuse(folder, &format!("it holds {}", guarded.display())));
        }
    }
    let meta = fs::symlink_metadata(folder).map_err(|e| InstallError::io(format!("looking at {}", folder.display()), &e))?;
    if meta.file_type().is_symlink() {
        // A link is taken away, not followed.
        return Ok(());
    }
    if !meta.is_dir() {
        return Err(refuse(folder, "it is not a folder"));
    }
    let marked = folder.join(layout::VERSION_FILE).exists() || folder.join(layout::INCOMPLETE_FILE).exists();
    if !marked && !layout::is_complete(folder, platform) {
        return Err(refuse(folder, "it does not look like something Reclaw installed"));
    }
    Ok(())
}

/// Delete an app's folder. Returns what was removed.
pub fn uninstall(folder: &Path, protected: &[PathBuf], platform: Platform) -> Result<(), InstallError> {
    check_removable(folder, protected, platform)?;
    let meta = fs::symlink_metadata(folder).map_err(|e| InstallError::io(format!("looking at {}", folder.display()), &e))?;
    let result = if meta.file_type().is_symlink() { fs::remove_file(folder) } else { fs::remove_dir_all(folder) };
    result.map_err(|e| InstallError::io(format!("removing {}", folder.display()), &e))?;
    tracing::info!(folder = %folder.display(), "uninstalled");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const LINUX: Platform = Platform::LinuxX64;

    fn app(root: &Path) -> PathBuf {
        let folder = root.join("apps/Game");
        fs::create_dir_all(folder.join("saves")).expect("dirs");
        fs::write(folder.join("version.txt"), "v1").expect("version");
        fs::write(folder.join("saves/a.sav"), "x").expect("save");
        folder
    }

    #[test]
    fn an_installed_app_is_removed_with_everything_in_it() {
        let dir = tempfile::tempdir().expect("dir");
        let folder = app(dir.path());
        uninstall(&folder, &[dir.path().join("home")], LINUX).expect("removed");
        assert!(!folder.exists());
        assert!(dir.path().join("apps").exists(), "the location stays");
    }

    #[test]
    fn a_failed_install_can_be_cleaned_up_too() {
        let dir = tempfile::tempdir().expect("dir");
        let folder = dir.path().join("apps/Game");
        fs::create_dir_all(&folder).expect("dirs");
        fs::write(folder.join(layout::INCOMPLETE_FILE), "v1").expect("marker");
        uninstall(&folder, &[], LINUX).expect("removed");
        assert!(!folder.exists());
    }

    #[test]
    fn a_folder_that_is_not_an_install_is_left_alone() {
        let dir = tempfile::tempdir().expect("dir");
        let folder = dir.path().join("apps/Documents");
        fs::create_dir_all(&folder).expect("dirs");
        fs::write(folder.join("thesis.docx"), "years of work").expect("file");
        let error = uninstall(&folder, &[], LINUX).expect_err("refused");
        assert!(matches!(&error, InstallError::Unsupported(m) if m.contains("does not look like")), "{error:?}");
        assert!(folder.join("thesis.docx").exists());
    }

    #[test]
    fn nothing_that_holds_a_protected_place_is_removed() {
        let dir = tempfile::tempdir().expect("dir");
        let folder = app(dir.path());
        let home = folder.join("deep/home");
        fs::create_dir_all(&home).expect("home");
        let error = uninstall(&folder, &[home], LINUX).expect_err("refused");
        assert!(matches!(&error, InstallError::Unsupported(m) if m.contains("it holds")), "{error:?}");
        assert!(folder.exists());
    }

    #[test]
    fn the_top_of_the_disk_and_odd_addresses_are_refused() {
        for folder in ["/", "/home", "/home/user", "relative/path", "/a/b/../c/d", "/a/b/./c"] {
            assert!(check_removable(Path::new(folder), &[], LINUX).is_err(), "{folder}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_is_removed_without_touching_the_folder() {
        let dir = tempfile::tempdir().expect("dir");
        let real = app(dir.path());
        let link = dir.path().join("apps/Link");
        std::os::unix::fs::symlink(&real, &link).expect("link");
        uninstall(&link, &[], LINUX).expect("removed");
        assert!(!link.exists() && real.join("saves/a.sav").exists());
    }

    #[test]
    fn a_missing_folder_is_an_io_error() {
        let dir = tempfile::tempdir().expect("dir");
        let error = uninstall(&dir.path().join("apps/Gone"), &[], LINUX).expect_err("missing");
        assert!(matches!(error, InstallError::Io { .. }), "{error:?}");
    }
}
