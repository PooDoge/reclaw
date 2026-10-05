//! Finding, fetching and installing a game's release: which release is current, which of its files fits this machine, how a
//! downloaded file becomes an app folder, and where it ended up.
//!
//! * `platform`, `names`, `matcher`, `policy`, `format`: which file of a release to take (ports of Quiver's rules, pure)
//! * `version`, `release`: telling releases apart, the common shape of GitHub's and GitLab's, which one to install (pure)
//! * `archive`: unpacking zip, tar.gz, tar.xz and 7z safely (RAR through a tool on the machine)
//! * `layout`, `programs`: the shape of an app folder, flattening a wrapper, merging over an install, finding what to start
//! * `book`: where each app was installed; `remove`: uninstalling, and refusing to delete what is not an install
//! * `install`: the whole job for one release: choose, download, unpack, move into place, write the version
//! * `source`: asking the two hosts for a project's releases; `error`: why anything here failed
//!
//! Nothing here draws or knows a window. The program's host calls it from worker threads and reports what happens.
pub mod archive;
pub mod book;
pub mod error;
pub mod format;
pub mod install;
pub mod layout;
pub mod matcher;
mod names;
pub mod platform;
pub mod policy;
pub mod programs;
pub mod release;
pub mod remove;
pub mod source;
pub mod version;

pub use book::InstallBook;
pub use error::InstallError;
pub use format::Format;
pub use install::{Installed, Installer, Plan, Request, Resolved, Stage, Step};
pub use platform::Platform;
pub use policy::Selection;
pub use release::{Asset, Release, select_release};
pub use source::{Api, Host, ReleaseSource, Releases};

#[cfg(test)]
mod tests;
