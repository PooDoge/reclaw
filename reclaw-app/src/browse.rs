//! Opening a link in the system browser. Only public https addresses are opened: a link comes from a catalog or a README written by
//! someone else, and it must not be able to start a program or open a file.
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use reclaw_net::AddressPolicy;

/// Why a link was not opened.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OpenError {
    #[error("{0} is not a link Reclaw will open: {1}")]
    Refused(String, reclaw_net::UrlError),
    #[error("the system's link opener could not be started ({0}); the link is {1}")]
    NoOpener(String, String),
    #[error("{0} is not a folder that exists")]
    NotAFolder(PathBuf),
}

/// The command that opens an address (or a path) on this system.
fn opener(url: impl AsRef<OsStr>) -> Command {
    if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url.as_ref());
        c
    } else if cfg!(target_os = "windows") {
        // `start` is a shell built-in; this handler is a program, and takes the address as one argument with no shell in between.
        let mut c = Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler").arg(url.as_ref());
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url.as_ref());
        c
    }
}

/// Check the address and hand it to the system.
pub fn open_url(url: &str) -> Result<(), OpenError> {
    let checked = AddressPolicy::Public.parse(url).map_err(|why| OpenError::Refused(url.to_string(), why))?;
    launch(opener(checked.as_str()), checked.as_str())
}

/// Show one of Reclaw's own folders (the logs) in the file manager. Only ever called with a path Reclaw made itself, never one
/// that came from a catalog.
pub fn open_folder(path: &Path) -> Result<(), OpenError> {
    if !path.is_dir() {
        return Err(OpenError::NotAFolder(path.to_path_buf()));
    }
    launch(opener(path), &path.display().to_string())
}

fn launch(mut command: Command, what: &str) -> Result<(), OpenError> {
    command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command.spawn().map_err(|e| OpenError::NoOpener(e.to_string(), what.to_string()))?;
    // Wait for it somewhere else so it does not stay a zombie; the opener returns as soon as the other program has it.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_public_https_links_are_opened() {
        for bad in [
            "file:///etc/passwd",
            "http://example.com",
            "javascript:alert(1)",
            "https://127.0.0.1/admin",
            "https://user:pw@example.com/",
            "ssh://host",
            "-oProxyCommand=x",
            "",
        ] {
            assert!(matches!(open_url(bad), Err(OpenError::Refused(..))), "{bad:?}");
        }
    }

    #[test]
    fn a_folder_that_is_not_there_is_refused_before_anything_is_started() {
        assert!(matches!(open_folder(Path::new("/nonexistent/reclaw/logs")), Err(OpenError::NotAFolder(_))));
    }

    #[test]
    fn the_address_is_one_argument_and_never_goes_through_a_shell() {
        let command = opener("https://example.com/a?b=1&c=2;rm");
        let args: Vec<_> = command.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert_eq!(args.last().map(String::as_str), Some("https://example.com/a?b=1&c=2;rm"));
        assert!(!command.get_program().to_string_lossy().contains("sh"), "{:?}", command.get_program());
    }
}
