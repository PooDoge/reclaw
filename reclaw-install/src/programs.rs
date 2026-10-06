//! Finding what to start in an app folder. Quiver's rules: on Linux an `.AppImage`, a `.x86_64` or a file that is an ELF
//! program counts, a `.sh` script counts, and a Windows `.exe` counts only when there is nothing native (it then needs a
//! compatibility layer). A file's name is not evidence of a program, its first bytes are: a `LICENSE` is not one, and a
//! zip that lost its execute bits still has the ELF header it was built with.
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{layout, platform::Platform};

/// What was found, nearest the top first.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Programs {
    pub programs: Vec<PathBuf>,
    /// Only Windows programs were found: something has to run them.
    pub needs_runner: bool,
}

/// A Wine or Proton prefix is a copy of a Windows machine, not part of the game; never look inside one.
fn is_compat_dir(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| matches!(n, ".steam-compat-data" | ".wine-prefix" | layout::STAGE_DIR))
        || (path.join("dosdevices").is_dir() && path.join("drive_c").is_dir())
}

/// Every file under `root` (links to folders are not followed), metadata aside.
fn files_under(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        if is_compat_dir(&dir) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else { continue };
            if meta.is_dir() {
                if recursive {
                    pending.push(path);
                }
            } else if !path.file_name().and_then(|n| n.to_str()).is_some_and(layout::is_metadata_file) {
                found.push(path);
            }
        }
    }
    found
}

fn lower_name(path: &Path) -> String {
    path.file_name().and_then(|n| n.to_str()).unwrap_or_default().to_ascii_lowercase()
}

fn depth(root: &Path, path: &Path) -> usize {
    path.strip_prefix(root).map(|p| p.components().count()).unwrap_or(usize::MAX)
}

/// A shebang line naming an interpreter by absolute path, or an ELF program (an executable or a position-independent one with
/// an entry point; a shared library has none).
pub fn is_native_executable(path: &Path) -> bool {
    let mut header = [0u8; 64];
    let Ok(mut file) = fs::File::open(path) else { return false };
    let Ok(length) = file.read(&mut header) else { return false };
    let header = &header[..length];
    if header.starts_with(b"#!") {
        return header[2..].iter().find(|b| !matches!(b, b' ' | b'\t')) == Some(&b'/');
    }
    if length < 52 || &header[..4] != b"\x7fELF" || !matches!(header[4], 1 | 2) || !matches!(header[5], 1 | 2) || header[6] != 1 {
        return false;
    }
    let is64 = header[4] == 2;
    if is64 && length < 64 {
        return false;
    }
    let kind = if header[5] == 1 { u16::from_le_bytes([header[16], header[17]]) } else { u16::from_be_bytes([header[16], header[17]]) };
    let entry = &header[24..24 + if is64 { 8 } else { 4 }];
    matches!(kind, 2 | 3) && entry.iter().any(|b| *b != 0)
}

/// The Mach-O magic numbers, for a macOS build.
fn is_mach_o(path: &Path) -> bool {
    let mut magic = [0u8; 4];
    let Ok(mut file) = fs::File::open(path) else { return false };
    file.read_exact(&mut magic).is_ok()
        && matches!(u32::from_be_bytes(magic), 0xFEED_FACE | 0xCEFA_EDFE | 0xFEED_FACF | 0xCFFA_EDFE | 0xCAFE_BABE | 0xBEBA_FECA)
}

/// The programs in `root`: just its top level when `recursive` is false.
pub fn find(root: &Path, recursive: bool, platform: Platform) -> Programs {
    if !root.is_dir() || is_compat_dir(root) {
        return Programs::default();
    }
    let files = files_under(root, recursive);
    let mut found: Vec<PathBuf> = Vec::new();
    let mut needs_runner = false;
    match platform {
        Platform::Windows => {
            found.extend(files.iter().filter(|f| ["exe", "bat", "cmd"].iter().any(|e| lower_name(f).ends_with(&format!(".{e}")))).cloned());
        }
        Platform::MacOs => {
            let mut pending = vec![root.to_path_buf()];
            while let Some(dir) = pending.pop() {
                let Ok(entries) = fs::read_dir(&dir) else { continue };
                for entry in entries.flatten() {
                    let path = entry.path();
                    if lower_name(&path).ends_with(".app") && path.is_dir() {
                        found.push(path);
                    } else if recursive && fs::symlink_metadata(&path).is_ok_and(|m| m.is_dir()) {
                        pending.push(path);
                    }
                }
            }
            found.extend(files.iter().filter(|f| !lower_name(f).contains('.') && is_mach_o(f)).cloned());
        }
        _ => {
            const OWN: [&str; 4] = [".x86_64", ".appimage", ".arm64", ".aarch64"];
            const NOT_PROGRAMS: [&str; 7] = [".txt", ".dll", ".so", ".json", ".sh", ".exe", ".md"];
            found.extend(files.iter().filter(|f| OWN.iter().any(|e| lower_name(f).ends_with(e))).cloned());
            found.extend(
                files
                    .iter()
                    .filter(|f| {
                        let name = lower_name(f);
                        !OWN.iter().chain(NOT_PROGRAMS.iter()).any(|e| name.ends_with(e))
                            && !name.contains(".so.")
                            && is_native_executable(f)
                    })
                    .cloned(),
            );
            let exes: Vec<PathBuf> = files.iter().filter(|f| lower_name(f).ends_with(".exe")).cloned().collect();
            // A Windows build that ships its sources can hold ELF files deep inside (a library's test fixtures, seen in a real
            // release): a native file only makes the app native when it is as near the top as the nearest `.exe`.
            let nearest = |paths: &[PathBuf]| paths.iter().map(|p| depth(root, p)).min();
            if let Some(exe_depth) = nearest(&exes)
                && nearest(&found).is_none_or(|native_depth| exe_depth < native_depth)
            {
                found = exes;
                needs_runner = true;
            }
            found.extend(files.iter().filter(|f| lower_name(f).ends_with(".sh")).cloned());
        }
    }
    found.sort_by(|a, b| depth(root, a).cmp(&depth(root, b)).then_with(|| lower_name(a).cmp(&lower_name(b))).then_with(|| a.cmp(b)));
    found.dedup();
    Programs { programs: found, needs_runner }
}

/// Make one file executable (adds the bits; never takes any away).
#[cfg(unix)]
pub fn make_file_runnable(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path)?.permissions().mode();
    if mode & 0o111 == 0o111 {
        return Ok(());
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode | 0o111))
}

#[cfg(not(unix))]
pub fn make_file_runnable(_path: &Path) -> std::io::Result<()> {
    Ok(())
}

/// Make the programs in `root` executable: an archive made on Windows keeps no execute bits, and the ELF files in it are
/// still programs. Adds the bits; never takes any away.
#[cfg(unix)]
pub fn make_runnable(root: &Path, platform: Platform) {
    use std::os::unix::fs::PermissionsExt;
    for program in find(root, true, platform).programs.iter().filter(|p| p.is_file()) {
        let Ok(meta) = fs::metadata(program) else { continue };
        let mode = meta.permissions().mode();
        if mode & 0o111 != 0o111
            && let Err(error) = fs::set_permissions(program, fs::Permissions::from_mode(mode | 0o111))
        {
            tracing::warn!(program = %program.display(), %error, "a program could not be made executable");
        }
    }
}

#[cfg(not(unix))]
pub fn make_runnable(_root: &Path, _platform: Platform) {}
