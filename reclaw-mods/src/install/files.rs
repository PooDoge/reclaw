//! Moving a mod's files in and out of the mods folder without leaving anything outside it or half done: every path is checked
//! to stay inside the folder (also through links on disk), a file that is replaced is set aside until the whole mod is in place,
//! and folders a removal empties are removed with it (the recomp runtime opens every folder in `mods` as a mod, and an empty one
//! is an error there).
use std::{
    fs,
    path::{Path, PathBuf},
};

use reclaw_install::archive::safe_relative;

use crate::error::ModError;

/// `relative` (`/`-separated) inside `root`, or `None` when it would leave it, by its name or through a link on disk.
pub fn inside(root: &Path, relative: &str) -> Option<PathBuf> {
    let path = root.join(safe_relative(relative)?);
    // The deepest part that exists must resolve inside the root, so a link planted in the mods folder cannot lead a write out.
    let real_root = fs::canonicalize(root).ok()?;
    let mut existing = path.parent()?;
    while !existing.exists() {
        existing = existing.parent()?;
    }
    fs::canonicalize(existing).ok()?.starts_with(&real_root).then_some(path)
}

/// Every regular file under `tree`, `/`-separated and sorted. Links are left out: a mod's files are files.
pub fn regular_files(tree: &Path) -> Result<Vec<String>, ModError> {
    let mut found = Vec::new();
    let mut pending = vec![tree.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| ModError::io(format!("reading {}", dir.display()), &e))?;
        for entry in entries {
            let entry = entry.map_err(|e| ModError::io(format!("reading {}", dir.display()), &e))?;
            let path = entry.path();
            let kind = entry.file_type().map_err(|e| ModError::io(format!("reading {}", path.display()), &e))?;
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let relative = path.strip_prefix(tree).unwrap_or(&path);
                found.push(relative.components().map(|c| c.as_os_str().to_string_lossy()).collect::<Vec<_>>().join("/"));
            } else {
                tracing::warn!(path = %path.display(), "a link or special file in a mod was left out");
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Files moved into the mods folder so far, and the files they replaced, so the move can be undone.
pub struct Moves {
    backup: PathBuf,
    placed: Vec<PathBuf>,
    replaced: Vec<(PathBuf, PathBuf)>,
}

impl Moves {
    /// `backup` is a folder on the same disk where replaced files wait.
    pub fn new(backup: PathBuf) -> Self {
        Self { backup, placed: Vec::new(), replaced: Vec::new() }
    }

    /// Move `from` to `to`, setting aside whatever file is at `to`.
    pub fn place(&mut self, from: &Path, to: &Path) -> Result<(), ModError> {
        match fs::symlink_metadata(to) {
            Ok(meta) if meta.is_dir() => {
                return Err(ModError::Io {
                    what: format!("placing {}", to.display()),
                    error: "a folder of that name is in the way".into(),
                });
            }
            Ok(_) => {
                let aside = self.backup.join(self.replaced.len().to_string());
                fs::create_dir_all(&self.backup).map_err(|e| ModError::io(format!("making {}", self.backup.display()), &e))?;
                fs::rename(to, &aside).map_err(|e| ModError::io(format!("setting aside {}", to.display()), &e))?;
                self.replaced.push((to.to_path_buf(), aside));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(ModError::io(format!("looking at {}", to.display()), &error)),
        }
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| ModError::io(format!("making {}", parent.display()), &e))?;
        }
        move_file(from, to)?;
        self.placed.push(to.to_path_buf());
        Ok(())
    }

    /// Put everything back as it was: placed files go (and the folders they made, if empty), replaced ones return.
    pub fn undo(self, root: &Path) {
        for path in self.placed.iter().rev() {
            if let Err(error) = fs::remove_file(path) {
                tracing::warn!(path = %path.display(), %error, "a file placed by a failed mod install could not be removed");
            }
            prune(root, path);
        }
        for (original, aside) in self.replaced.iter().rev() {
            if let Err(error) = fs::rename(aside, original) {
                tracing::error!(path = %original.display(), aside = %aside.display(), %error, "a replaced file could not be put back");
            }
        }
    }

    /// The move stands: the replaced files are no longer needed.
    pub fn keep(self) {
        if self.backup.exists()
            && let Err(error) = fs::remove_dir_all(&self.backup)
        {
            tracing::warn!(path = %self.backup.display(), %error, "replaced mod files could not be cleared away");
        }
    }
}

/// A rename, or a copy and delete when the two are on different disks.
fn move_file(from: &Path, to: &Path) -> Result<(), ModError> {
    if fs::rename(from, to).is_ok() {
        return Ok(());
    }
    fs::copy(from, to).map_err(|e| ModError::io(format!("copying to {}", to.display()), &e))?;
    if let Err(error) = fs::remove_file(from) {
        tracing::debug!(path = %from.display(), %error, "a staged mod file stayed behind after copying");
    }
    Ok(())
}

/// Remove `relative` from `root` if it is a file there, and the folders that leaves empty. A missing file is not an error.
pub fn remove(root: &Path, relative: &str) -> Result<(), ModError> {
    if !root.exists() {
        return Ok(());
    }
    let Some(path) = inside(root, relative) else {
        tracing::warn!(root = %root.display(), relative, "a recorded mod file is outside the mods folder; it was not touched");
        return Ok(());
    };
    match fs::symlink_metadata(&path) {
        Ok(meta) if meta.is_dir() => {
            tracing::warn!(path = %path.display(), "a recorded mod file is now a folder; it was not touched");
            Ok(())
        }
        Ok(_) => {
            fs::remove_file(&path).map_err(|e| ModError::io(format!("removing {}", path.display()), &e))?;
            prune(root, &path);
            Ok(())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            prune(root, &path);
            Ok(())
        }
        Err(error) => Err(ModError::io(format!("looking at {}", path.display()), &error)),
    }
}

/// Remove the empty folders above `path`, up to (not including) `root`.
pub fn prune(root: &Path, path: &Path) {
    let mut dir = path.parent();
    while let Some(current) = dir {
        if current == root || !current.starts_with(root) {
            break;
        }
        // `remove_dir` refuses a folder that is not empty, which is the test.
        if fs::remove_dir(current).is_err() {
            break;
        }
        dir = current.parent();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_stay_inside_the_folder() {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().join("mods");
        fs::create_dir_all(&root).expect("mkdir");
        assert_eq!(inside(&root, "a/b.nrm"), Some(root.join("a/b.nrm")));
        assert_eq!(inside(&root, "../escape.nrm"), None);
        assert_eq!(inside(&root, "/etc/passwd"), Some(root.join("etc/passwd")), "a leading slash is read inside the folder");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(dir.path(), root.join("out")).expect("link");
            assert_eq!(inside(&root, "out/x.nrm"), None, "not through a link that leads out");
        }
    }

    #[test]
    fn a_failed_move_puts_everything_back() {
        let dir = tempfile::tempdir().expect("dir");
        let (root, stage) = (dir.path().join("mods"), dir.path().join("stage"));
        fs::create_dir_all(&root).expect("mkdir");
        fs::create_dir_all(&stage).expect("mkdir");
        fs::write(root.join("old.nrm"), "old").expect("write");
        fs::write(stage.join("a"), "new").expect("write");
        fs::write(stage.join("b"), "deep").expect("write");
        let mut moves = Moves::new(dir.path().join("backup"));
        moves.place(&stage.join("a"), &root.join("old.nrm")).expect("replace");
        moves.place(&stage.join("b"), &root.join("sub/deep.nrm")).expect("place");
        assert_eq!(fs::read_to_string(root.join("old.nrm")).expect("read"), "new");
        moves.undo(&root);
        assert_eq!(fs::read_to_string(root.join("old.nrm")).expect("read"), "old");
        assert!(!root.join("sub").exists(), "the folder it made is gone");
    }

    #[test]
    fn removing_a_file_removes_the_folders_it_leaves_empty() {
        let dir = tempfile::tempdir().expect("dir");
        let root = dir.path().join("mods");
        fs::create_dir_all(root.join("a/b")).expect("mkdir");
        fs::write(root.join("a/b/x.nrm"), "x").expect("write");
        fs::write(root.join("a/keep.txt"), "k").expect("write");
        remove(&root, "a/b/x.nrm").expect("removes");
        assert!(!root.join("a/b").exists() && root.join("a/keep.txt").exists() && root.exists());
        remove(&root, "a/b/x.nrm").expect("already gone is fine");
        remove(&root, "../outside").expect("refused quietly");
        assert_eq!(regular_files(&root).expect("lists"), vec!["a/keep.txt".to_string()]);
    }
}
