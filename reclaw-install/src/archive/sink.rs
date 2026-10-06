//! Where an archive's entries are written. Every format reads its own entries and hands them here, so the rules that keep a
//! hostile archive inside its folder exist once: no entry may leave the folder (`..`, absolute paths, drive letters), nothing
//! is written *through* a link an earlier entry made, a link may only point inside the folder, set-id bits are dropped, and the
//! total size and number of entries are capped (a few kilobytes of archive can describe terabytes).
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

use reclaw_net::Cancel;

use crate::error::InstallError;

/// Upper bounds on what one archive may unpack to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_entries: usize,
}

impl Default for Limits {
    /// Large enough for any game; small enough that a zip bomb is stopped by this and not by the disk filling.
    fn default() -> Self {
        Self { max_bytes: 64 * 1024 * 1024 * 1024, max_entries: 500_000 }
    }
}

/// How far an extraction has got. `total` is in the same unit as `done` (unpacked bytes, or the archive's own bytes for a
/// stream whose unpacked size is not known in advance).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Position {
    pub done: u64,
    pub total: Option<u64>,
}

const REPORT_EVERY: Duration = Duration::from_millis(100);
const CHUNK: usize = 256 * 1024;

pub struct Sink<'a> {
    root: PathBuf,
    limits: Limits,
    cancel: &'a Cancel,
    on_progress: &'a mut dyn FnMut(Position),
    total: Option<u64>,
    bytes: u64,
    entries: usize,
    /// Entries that were left out (a device file, a hard link), for the log.
    pub skipped: usize,
    last_report: Instant,
    /// The last folder checked for links, so a thousand files in one folder cost one check.
    checked_parent: Option<PathBuf>,
}

/// An entry's name as a path inside the folder, or `None` when it would leave it. Backslashes are separators (archives made
/// on Windows), and `.` components and empty ones are dropped.
pub fn safe_relative(name: &str) -> Option<PathBuf> {
    let name = name.replace('\\', "/");
    let mut clean = PathBuf::new();
    for part in name.split('/') {
        match Path::new(part).components().next() {
            None | Some(Component::CurDir) => {}
            Some(Component::Normal(piece)) if Path::new(part).components().count() == 1 && !part.contains('\0') => clean.push(piece),
            // `..`, a drive (`C:`), a root: all ways out.
            _ => return None,
        }
    }
    (!clean.as_os_str().is_empty()).then_some(clean)
}

/// Where a link at `at` (relative to the root) lands if followed, written out lexically; `None` when it leaves the root.
fn lexical_target(at: &Path, target: &str) -> Option<PathBuf> {
    let target = target.replace('\\', "/");
    if target.starts_with('/') || target.contains('\0') {
        return None;
    }
    let mut parts: Vec<std::ffi::OsString> = at.parent().map(|p| p.iter().map(ToOwned::to_owned).collect()).unwrap_or_default();
    for piece in target.split('/') {
        match piece {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            other => {
                if Path::new(other).components().count() != 1 || matches!(Path::new(other).components().next(), Some(Component::Prefix(_)))
                {
                    return None;
                }
                parts.push(other.into());
            }
        }
    }
    Some(parts.into_iter().collect())
}

impl<'a> Sink<'a> {
    pub fn new(root: &Path, limits: Limits, total: Option<u64>, cancel: &'a Cancel, on_progress: &'a mut dyn FnMut(Position)) -> Self {
        Self {
            root: root.to_path_buf(),
            limits,
            cancel,
            on_progress,
            total,
            bytes: 0,
            entries: 0,
            skipped: 0,
            last_report: Instant::now() - REPORT_EVERY,
            checked_parent: None,
        }
    }

    pub fn entries(&self) -> usize {
        self.entries
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    fn check_cancel(&self) -> Result<(), InstallError> {
        if self.cancel.is_cancelled() { Err(InstallError::Cancelled) } else { Ok(()) }
    }

    fn count_entry(&mut self) -> Result<(), InstallError> {
        self.check_cancel()?;
        self.entries += 1;
        if self.entries > self.limits.max_entries {
            return Err(InstallError::Archive(format!(
                "the archive has more than {} entries; it is refused as a likely bomb",
                self.limits.max_entries
            )));
        }
        Ok(())
    }

    /// Report progress in the archive's own unit (a stream that cannot say how big it will be once unpacked).
    pub fn report(&mut self, done: u64, total: Option<u64>) {
        if self.last_report.elapsed() >= REPORT_EVERY {
            self.last_report = Instant::now();
            (self.on_progress)(Position { done, total });
        }
    }

    fn resolve(&self, name: &str) -> Result<(PathBuf, PathBuf), InstallError> {
        let relative = safe_relative(name)
            .ok_or_else(|| InstallError::Archive(format!("the archive has an entry that would leave its folder: {name}")))?;
        Ok((self.root.join(&relative), relative))
    }

    /// Make every folder above `relative` and make sure none of them is a link an earlier entry made.
    fn prepare_parents(&mut self, relative: &Path) -> Result<(), InstallError> {
        let Some(parent) = relative.parent().filter(|p| !p.as_os_str().is_empty()) else { return Ok(()) };
        if self.checked_parent.as_deref() == Some(parent) {
            return Ok(());
        }
        let mut walked = self.root.clone();
        for piece in parent.iter() {
            walked.push(piece);
            match fs::symlink_metadata(&walked) {
                Ok(meta) if meta.file_type().is_symlink() => {
                    return Err(InstallError::Archive(format!("the archive writes through a link: {}", relative.display())));
                }
                Ok(meta) if !meta.is_dir() => {
                    return Err(InstallError::Archive(format!("the archive has a file where a folder is needed: {}", walked.display())));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fs::create_dir(&walked).map_err(|e| InstallError::io(format!("making {}", walked.display()), &e))?;
                }
                Err(error) => return Err(InstallError::io(format!("looking at {}", walked.display()), &error)),
            }
        }
        self.checked_parent = Some(parent.to_path_buf());
        Ok(())
    }

    pub fn dir(&mut self, name: &str, mode: Option<u32>) -> Result<(), InstallError> {
        let Some(relative) = safe_relative(name) else {
            // The archive's own top folder entry (`./`) is not an entry to make.
            return if name.trim_matches(['/', '.', '\\']).is_empty() {
                Ok(())
            } else {
                Err(InstallError::Archive(format!("the archive has an entry that would leave its folder: {name}")))
            };
        };
        self.count_entry()?;
        self.prepare_parents(&relative)?;
        let path = self.root.join(&relative);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(InstallError::Archive(format!("the archive writes through a link: {name}")));
            }
            Ok(_) => {}
            Err(_) => fs::create_dir(&path).map_err(|e| InstallError::io(format!("making {}", path.display()), &e))?,
        }
        // Owner access is kept whatever the archive says: a folder we cannot enter cannot be filled.
        set_mode(&path, mode.map(|m| m | 0o700));
        Ok(())
    }

    pub fn file(&mut self, name: &str, mode: Option<u32>, data: &mut dyn Read) -> Result<(), InstallError> {
        let (path, relative) = self.resolve(name)?;
        self.count_entry()?;
        self.prepare_parents(&relative)?;
        if fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink() || m.is_dir()) {
            remove_any(&path)?;
        }
        let mut out = fs::File::create(&path).map_err(|e| InstallError::io(format!("writing {}", path.display()), &e))?;
        let mut buffer = vec![0u8; CHUNK];
        loop {
            self.check_cancel()?;
            let n = data.read(&mut buffer).map_err(|e| InstallError::Archive(format!("{name} could not be unpacked: {e}")))?;
            if n == 0 {
                break;
            }
            self.bytes += n as u64;
            if self.bytes > self.limits.max_bytes {
                return Err(InstallError::Archive(format!(
                    "the archive unpacks to more than {} GiB; it is refused as a likely bomb",
                    self.limits.max_bytes >> 30
                )));
            }
            out.write_all(&buffer[..n]).map_err(|e| InstallError::io(format!("writing {}", path.display()), &e))?;
            let (done, total) = (self.bytes, self.total);
            self.report(done, total);
        }
        out.flush().map_err(|e| InstallError::io(format!("writing {}", path.display()), &e))?;
        drop(out);
        // Owner can always read and write what we unpacked; execute bits survive; set-id bits do not.
        set_mode(&path, mode.map(|m| m | 0o600));
        Ok(())
    }

    /// A link, if it stays inside the folder; otherwise it is left out (and counted) rather than failing the whole install.
    pub fn symlink(&mut self, name: &str, target: &str) -> Result<(), InstallError> {
        let (path, relative) = self.resolve(name)?;
        self.count_entry()?;
        if lexical_target(&relative, target).is_none() {
            tracing::warn!(entry = name, target, "a link in the archive points outside its folder and was left out");
            self.skipped += 1;
            return Ok(());
        }
        self.prepare_parents(&relative)?;
        if fs::symlink_metadata(&path).is_ok() {
            remove_any(&path)?;
        }
        make_symlink(target, &path)
    }

    pub fn skip(&mut self, name: &str, why: &str) {
        tracing::warn!(entry = name, why, "an archive entry was left out");
        self.skipped += 1;
    }
}

fn remove_any(path: &Path) -> Result<(), InstallError> {
    let meta = fs::symlink_metadata(path).map_err(|e| InstallError::io(format!("looking at {}", path.display()), &e))?;
    let result = if meta.is_dir() && !meta.file_type().is_symlink() { fs::remove_dir_all(path) } else { fs::remove_file(path) };
    result.map_err(|e| InstallError::io(format!("replacing {}", path.display()), &e))
}

#[cfg(unix)]
fn set_mode(path: &Path, mode: Option<u32>) {
    use std::os::unix::fs::PermissionsExt;
    if let Some(mode) = mode {
        // Failing to set a mode leaves the default one, which is usable; it is worth a line in the log, not a failed install.
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o777)) {
            tracing::debug!(path = %path.display(), %error, "permissions could not be set");
        }
    }
}

#[cfg(not(unix))]
fn set_mode(_path: &Path, _mode: Option<u32>) {}

#[cfg(unix)]
fn make_symlink(target: &str, path: &Path) -> Result<(), InstallError> {
    std::os::unix::fs::symlink(target, path).map_err(|e| InstallError::io(format!("linking {}", path.display()), &e))
}

#[cfg(not(unix))]
fn make_symlink(_target: &str, path: &Path) -> Result<(), InstallError> {
    tracing::warn!(path = %path.display(), "links are not made on this system");
    Ok(())
}
