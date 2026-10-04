//! Writing a file so a crash or a full disk never leaves half of it.
use std::{
    ffi::OsString,
    fs,
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU32, Ordering},
};

use super::ConfigEditError;

/// Write `text` to `path` through a temporary file in the same directory and a rename, creating
/// parent directories. The first time a file is changed its original is kept beside it as
/// `<name>.reclaw-orig`.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), ConfigEditError> {
    let io = |source| ConfigEditError::io(path, source);
    let name = path.file_name().ok_or_else(|| ConfigEditError::bad_path(&path.display().to_string(), "it names no file"))?;
    let dir = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(io)?;

    let existing = match fs::metadata(path) {
        Ok(meta) if meta.is_file() => Some(meta),
        // A directory (or anything else) in the way is reported by the rename below.
        Ok(_) => None,
        Err(e) if e.kind() == ErrorKind::NotFound => None,
        Err(e) => return Err(io(e)),
    };
    if existing.is_some() {
        let mut backup: OsString = name.to_owned();
        backup.push(".reclaw-orig");
        let backup = dir.join(backup);
        if !backup.try_exists().map_err(io)? {
            fs::copy(path, &backup).map_err(io)?;
        }
    }

    let temporary = temporary_beside(dir, name);
    let written = write_file(&temporary, text).and_then(|()| match &existing {
        Some(meta) => fs::set_permissions(&temporary, meta.permissions()),
        None => Ok(()),
    });
    let renamed = written.and_then(|()| fs::rename(&temporary, path));
    if let Err(source) = renamed {
        // The temporary file is ours alone; the target was not touched.
        let _ = fs::remove_file(&temporary);
        return Err(ConfigEditError::io(path, source));
    }
    Ok(())
}

fn write_file(path: &Path, text: &str) -> std::io::Result<()> {
    let mut file = fs::OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()
}

/// A name nobody else is using: two launches editing the same game never share a temporary file.
fn temporary_beside(dir: &Path, name: &std::ffi::OsStr) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let mut file = OsString::from(".");
    file.push(name);
    file.push(format!(".reclaw-tmp-{}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
    dir.join(file)
}
