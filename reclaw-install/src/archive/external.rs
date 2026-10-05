//! Formats this program cannot read itself (RAR, and the rare zip method it lacks) are handed to a tool already on the machine,
//! if there is one. The tool is run with its output quiet and its answer checked; whatever it unpacked is then walked once more
//! to remove links that point outside the folder, because a tool's idea of a safe link is not ours.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{error::InstallError, format::Format};

/// How to run a tool to unpack `archive` into `dest`.
struct Tool {
    program: &'static str,
    args: fn(&Path, &Path) -> Vec<std::ffi::OsString>,
}

fn dest_flag(prefix: &str, dest: &Path) -> std::ffi::OsString {
    let mut flag = std::ffi::OsString::from(prefix);
    flag.push(dest);
    flag
}

fn tools_for(format: Format) -> Vec<Tool> {
    // 7-Zip reads everything, so it is the answer to any format; the others are what a system is likelier to have.
    let sevenz = |program| Tool { program, args: |a, d| vec!["x".into(), "-y".into(), dest_flag("-o", d), a.into()] };
    let bsdtar = Tool { program: "bsdtar", args: |a, d| vec!["-xf".into(), a.into(), "-C".into(), d.into()] };
    match format {
        Format::Rar => vec![
            Tool {
                program: "unrar",
                args: |a, d| {
                    vec!["x".into(), "-o+".into(), "-idq".into(), a.into(), {
                        let mut dir = d.as_os_str().to_owned();
                        dir.push("/");
                        dir
                    }]
                },
            },
            sevenz("7zz"),
            sevenz("7z"),
            sevenz("7za"),
            bsdtar,
        ],
        Format::Zip => vec![
            Tool { program: "unzip", args: |a, d| vec!["-qq".into(), "-o".into(), a.into(), "-d".into(), d.into()] },
            sevenz("7zz"),
            sevenz("7z"),
            bsdtar,
        ],
        _ => vec![sevenz("7zz"), sevenz("7z"), bsdtar],
    }
}

/// Look for `program` on the search path.
fn find_on_path(program: &str) -> Option<PathBuf> {
    std::env::var_os("PATH")
        .and_then(|paths| std::env::split_paths(&paths).map(|dir| dir.join(program)).find(|candidate| candidate.is_file()))
}

/// Unpack with a tool from the system. The error names what was tried and what to install.
pub fn extract(format: Format, archive: &Path, dest: &Path) -> Result<(), InstallError> {
    let tools = tools_for(format);
    let mut tried = Vec::new();
    for tool in &tools {
        let Some(path) = find_on_path(tool.program) else { continue };
        tried.push(tool.program);
        let output =
            Command::new(&path).args((tool.args)(archive, dest)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::piped()).output();
        match output {
            Ok(done) if done.status.success() => {
                tracing::info!(tool = tool.program, "unpacked with a tool from the system");
                remove_escaping_links(dest);
                return Ok(());
            }
            Ok(done) => {
                let stderr = String::from_utf8_lossy(&done.stderr);
                tracing::warn!(tool = tool.program, status = ?done.status.code(), stderr = %stderr.trim().chars().take(300).collect::<String>(), "a tool could not unpack the archive");
            }
            Err(error) => tracing::warn!(tool = tool.program, %error, "a tool could not be run"),
        }
    }
    let kind = match format {
        Format::Rar => "RAR",
        Format::SevenZip => "7z",
        Format::Zip => "zip",
        _ => "archive",
    };
    Err(InstallError::Unsupported(if tried.is_empty() {
        format!("Reclaw cannot unpack this {kind} file itself, and none of unrar, 7z or bsdtar is installed to do it for Reclaw.")
    } else {
        format!("This {kind} file could not be unpacked (tried {}).", tried.join(", "))
    }))
}

/// Remove links under `root` that point outside it, or at an absolute path.
fn remove_escaping_links(root: &Path) {
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::symlink_metadata(&path) else { continue };
            if meta.file_type().is_symlink() {
                let inside = fs::read_link(&path).ok().is_some_and(|target| {
                    !target.is_absolute() && {
                        let base = path.parent().unwrap_or(root);
                        crate::layout::normalize(&base.join(&target)).starts_with(crate::layout::normalize(root))
                    }
                });
                if !inside {
                    tracing::warn!(link = %path.display(), "a link pointing outside the folder was removed");
                    let _ = fs::remove_file(&path);
                }
            } else if meta.is_dir() {
                pending.push(path);
            }
        }
    }
}
