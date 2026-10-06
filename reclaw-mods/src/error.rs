//! Why listing, installing or removing a mod did not happen, in words a person can act on.
use reclaw_install::InstallError;
use reclaw_net::NetError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ModError {
    #[error("{0}")]
    Net(NetError),
    /// A site answered with something this program cannot read.
    #[error("{0}")]
    BadAnswer(String),
    /// The mod offers nothing to download.
    #[error("{0}")]
    NoDownload(String),
    /// Unpacking failed (a broken or hostile archive, a format with no reader).
    #[error("{0}")]
    Unpack(InstallError),
    #[error("{what}: {error}")]
    Io { what: String, error: String },
    /// The installed-mods file exists but cannot be read. Nothing is changed until it is fixed, so no record is lost.
    #[error("the list of installed mods ({path}) cannot be read: {error}")]
    Unreadable { path: String, error: String },
    /// Files of the mod would replace files another installed mod put there.
    #[error("{name} would replace {count} file(s) of {owner}", count = files.len())]
    Conflict { name: String, owner: String, files: Vec<String> },
    /// The app has no mods folder configured, or one that is not a plain path inside it.
    #[error("this app has no mods folder")]
    NoModsFolder,
    #[error("the download had nothing to install")]
    Empty,
    #[error("cancelled")]
    Cancelled,
}

impl ModError {
    pub fn io(what: impl Into<String>, error: &std::io::Error) -> Self {
        match InstallError::io(what, error) {
            InstallError::Io { what, error } => Self::Io { what, error },
            other => Self::Io { what: other.to_string(), error: error.to_string() },
        }
    }

    /// One sentence on what to do about it, when there is something to do.
    pub fn hint(&self) -> Option<String> {
        match self {
            Self::Net(error) => error.hint().map(str::to_string),
            Self::Unpack(error) => error.hint(),
            Self::Unreadable { .. } => Some("Fix the file or move it aside; Reclaw will not write over it.".to_string()),
            Self::Conflict { owner, .. } => Some(format!("Remove {owner} first if you want this one instead.")),
            Self::Io { error, .. } if error == "the disk is full" => Some("Free some disk space and try again.".to_string()),
            Self::Io { .. } => Some("Check that the game's folder is writable.".to_string()),
            _ => None,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        match self {
            Self::Cancelled | Self::Net(NetError::Cancelled) => true,
            Self::Unpack(error) => error.is_cancelled(),
            _ => false,
        }
    }
}

impl From<NetError> for ModError {
    fn from(error: NetError) -> Self {
        if matches!(error, NetError::Cancelled) { Self::Cancelled } else { Self::Net(error) }
    }
}

impl From<InstallError> for ModError {
    fn from(error: InstallError) -> Self {
        match error {
            InstallError::Net(net) => Self::from(net),
            InstallError::Cancelled => Self::Cancelled,
            InstallError::Io { what, error } => Self::Io { what, error },
            other => Self::Unpack(other),
        }
    }
}
