//! Why an install, an update or a release lookup did not happen, in words a person can act on.
use reclaw_net::NetError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum InstallError {
    /// The network said no (see [`NetError::hint`] for what to do).
    #[error("{0}")]
    Net(NetError),
    #[error("{repo} has no releases")]
    NoReleases { repo: String },
    /// A host answered with something that is not a list of releases.
    #[error("{0}")]
    BadAnswer(String),
    /// A release exists but none of its files fits (the policy's reason).
    #[error("{0}")]
    NoDownload(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Archive(String),
    /// A file or folder could not be read, made or moved.
    #[error("{what}: {error}")]
    Io { what: String, error: String },
    /// Everything was unpacked and nothing in it can be started.
    #[error("the download did not contain a program Reclaw can start")]
    NoProgram,
    #[error("cancelled")]
    Cancelled,
}

impl InstallError {
    pub fn io(what: impl Into<String>, error: &std::io::Error) -> Self {
        // A full disk is the one failure whose cure is not "try again".
        let text = if matches!(error.kind(), std::io::ErrorKind::StorageFull | std::io::ErrorKind::QuotaExceeded) {
            "the disk is full".to_string()
        } else {
            error.to_string()
        };
        Self::Io { what: what.into(), error: text }
    }

    /// One sentence on what to do about it, when there is something to do.
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::Net(error) => error.hint().map(str::to_string),
            Self::NoReleases { .. } => Some("The author has not published a release yet. Check the repository's page.".to_string()),
            Self::Io { error, .. } if error == "the disk is full" => {
                Some("Free some disk space, or choose another install location in Settings.".to_string())
            }
            Self::Io { .. } => Some("Check that the install location exists, is writable, and has room.".to_string()),
            Self::NoProgram => Some("The download may be incomplete, or this release is not a program for this system.".to_string()),
            _ => None,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancelled | Self::Net(NetError::Cancelled))
    }
}

impl From<NetError> for InstallError {
    fn from(error: NetError) -> Self {
        Self::Net(error)
    }
}
