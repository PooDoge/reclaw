//! Installing one release of one app: choose the release and the file, download it, unpack it beside the app, move it over
//! the installed copy, and write the version last. Quiver's order of work and its guarantees, not its UI:
//!
//! * a first install that does not finish leaves `install-incomplete.txt`, so the folder is not mistaken for an install;
//! * an update lays the new files over the old ones and leaves everything else (saves, settings) alone;
//! * `version.txt` is written when everything else is in place.
use std::{
    fs,
    path::{Path, PathBuf},
};

use reclaw_net::{Cancel, DownloadRequest, Net, NetError};

use crate::{
    archive::{self, Limits},
    error::InstallError,
    format::Format,
    layout::{self, INCOMPLETE_FILE, STAGE_DIR, VERSION_FILE},
    platform::Platform,
    policy::Selection,
    programs,
    release::{Asset, Release, select_release},
    source::{Host, ReleaseSource},
};

/// What to install and where.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Request {
    pub host: Host,
    /// `owner/name`.
    pub repo: String,
    /// The app's own folder (inside the install location).
    pub folder: PathBuf,
    pub platform: Platform,
    /// The catalog's `releaseAssetFilter`.
    pub filter: Option<String>,
    /// A release the catalog pins.
    pub preferred_version: Option<String>,
    pub allow_prerelease: bool,
    /// The name of a file the person chose when asked which one.
    pub asset: Option<String>,
}

/// Where the work is, for the screens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Downloading,
    Extracting,
    Finishing,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    pub stage: Stage,
    pub done: u64,
    pub total: Option<u64>,
    /// Bytes a second, while downloading.
    pub rate: Option<u64>,
}

/// What looking up the release found.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Plan {
    /// Install this file of this release.
    Ready(Box<Resolved>),
    /// Several files qualify and none is clearly the one: ask.
    Choose { release: Box<Release>, choices: Vec<Asset> },
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Resolved {
    pub release: Release,
    pub asset: Asset,
    /// The folder already holds a finished install of exactly this release.
    pub already_installed: bool,
}

/// What an install left behind.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Installed {
    pub folder: PathBuf,
    pub version: String,
    pub asset: String,
    /// The program Reclaw would start, nearest the top of the folder.
    pub program: Option<PathBuf>,
    pub needs_runner: bool,
    /// The release's own notes, for "what changed".
    pub notes: String,
    pub page: String,
}

#[derive(Clone)]
pub struct Installer {
    source: ReleaseSource,
    net: Net,
    /// Where downloads are kept until they are installed; a cut-off one continues from here.
    downloads: PathBuf,
    limits: Limits,
}

/// A name that is one path component, safe to make a file of.
fn file_name_of(name: &str) -> String {
    let cleaned: String = Path::new(name.trim())
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_control() || matches!(c, '/' | '\\' | ':') { '_' } else { c })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." { "download.bin".to_string() } else { cleaned }
}

/// FNV-1a: steady across Rust releases, unlike the standard hasher, so a cut-off download is found again after an update.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

fn io(what: &str, path: &Path, error: &std::io::Error) -> InstallError {
    InstallError::io(format!("{what} {}", path.display()), error)
}

impl Installer {
    pub fn new(net: Net, source: ReleaseSource, downloads: PathBuf) -> Self {
        Self { source, net, downloads, limits: Limits::default() }
    }

    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }

    pub fn releases(&self) -> &ReleaseSource {
        &self.source
    }

    /// Find the release and the file to install, without downloading anything.
    pub fn resolve(&self, request: &Request) -> Result<Plan, InstallError> {
        let whole_list = request.allow_prerelease || request.preferred_version.is_some();
        let found = self.source.fetch(request.host, &request.repo, whole_list)?;
        let release =
            select_release(&found.list, request.preferred_version.as_deref(), found.latest_tag.as_deref(), request.allow_prerelease)
                .ok_or_else(|| InstallError::NoDownload("No release of this app has files to install.".to_string()))?;
        let selection = Selection::of(release, request.platform, request.filter.as_deref());

        let chosen = match request.asset.as_deref() {
            Some(name) => Some(
                selection
                    .choices()
                    .iter()
                    .find(|a| a.name == name)
                    .cloned()
                    .ok_or_else(|| InstallError::NoDownload("That download is no longer available for this release.".to_string()))?,
            ),
            None => selection.automatic(request.platform).cloned(),
        };
        let Some(asset) = chosen else {
            if selection.needs_choice(request.platform) {
                return Ok(Plan::Choose { release: Box::new(release.clone()), choices: selection.choices().to_vec() });
            }
            return Err(InstallError::NoDownload(
                selection.reason.unwrap_or_else(|| "This release has no download for this system.".to_string()),
            ));
        };
        let already_installed = layout::installed_version(&request.folder).is_some_and(|v| v == release.tag.trim())
            && layout::is_complete(&request.folder, request.platform);
        Ok(Plan::Ready(Box::new(Resolved { release: release.clone(), asset, already_installed })))
    }

    fn download_to(&self, asset: &Asset) -> PathBuf {
        self.downloads.join(format!("{:016x}", fnv(&asset.url))).join(file_name_of(&asset.name))
    }

    /// Download, unpack and install `resolved` into `request.folder`.
    pub fn install(
        &self,
        request: &Request,
        resolved: &Resolved,
        cancel: &Cancel,
        on_step: &mut dyn FnMut(Step),
    ) -> Result<Installed, InstallError> {
        let (folder, asset, tag) = (&request.folder, &resolved.asset, resolved.release.tag.trim());
        tracing::info!(repo = %request.repo, tag, asset = %asset.name, folder = %folder.display(), "installing");
        let download = self.download_to(asset);
        let result = self.install_inner(request, resolved, &download, cancel, on_step);
        let stage = folder.join(STAGE_DIR);
        if stage.exists() {
            let _ = fs::remove_dir_all(&stage);
        }
        match &result {
            Ok(done) => {
                // The file did its job; a cut-off or failed install keeps it so the next try does not fetch it again.
                if let Some(dir) = download.parent() {
                    let _ = fs::remove_dir_all(dir);
                }
                tracing::info!(repo = %request.repo, tag, program = ?done.program, "installed");
            }
            Err(error) if error.is_cancelled() => tracing::info!(repo = %request.repo, tag, "the install was cancelled"),
            Err(error) => tracing::warn!(repo = %request.repo, tag, asset = %asset.name, %error, "the install failed"),
        }
        result
    }

    fn install_inner(
        &self,
        request: &Request,
        resolved: &Resolved,
        download: &Path,
        cancel: &Cancel,
        on_step: &mut dyn FnMut(Step),
    ) -> Result<Installed, InstallError> {
        let (folder, asset, tag) = (&request.folder, &resolved.asset, resolved.release.tag.trim());
        fs::create_dir_all(folder).map_err(|e| io("making", folder, &e))?;

        let mut download_request = DownloadRequest::new(&asset.url, download);
        if let Some(size) = asset.size.filter(|s| *s > 0) {
            download_request = download_request.size(size);
        }
        if let Some(digest) = &asset.sha256 {
            download_request = download_request.sha256(digest);
        }
        let fetched = self
            .net
            .download(&download_request, cancel, &mut |p| {
                on_step(Step { stage: Stage::Downloading, done: p.downloaded, total: p.total, rate: Some(p.bytes_per_sec as u64) });
            })
            .map_err(|error| if matches!(error, NetError::Cancelled) { InstallError::Cancelled } else { InstallError::Net(error) })?;

        let format = detect_format(&asset.name, &fetched.path)?;
        if !format.is_installable() {
            return Err(InstallError::Unsupported(format!("{} is {}, which Reclaw does not install.", asset.name, describe(format))));
        }

        // From here the folder changes. A first install says so until it is done.
        let first_install = !layout::is_complete(folder, request.platform);
        if first_install {
            fs::write(folder.join(INCOMPLETE_FILE), tag).map_err(|e| io("writing in", folder, &e))?;
        }

        let scratch = folder.join(STAGE_DIR);
        let tree = scratch.join("tree");
        if scratch.exists() {
            fs::remove_dir_all(&scratch).map_err(|e| io("clearing", &scratch, &e))?;
        }
        fs::create_dir_all(&tree).map_err(|e| io("making", &tree, &e))?;

        if format.is_archive() {
            self.unpack(format, &fetched.path, &tree, &scratch, cancel, on_step)?;
            if let Some(inner) = lone_tar_gz(&tree) {
                // An archive holding only a tar.gz is a wrapper around the real thing.
                let again = scratch.join("tree-inner");
                fs::create_dir_all(&again).map_err(|e| io("making", &again, &e))?;
                self.unpack(Format::TarGz, &inner, &again, &scratch, cancel, on_step)?;
                fs::remove_dir_all(&tree).map_err(|e| io("clearing", &tree, &e))?;
                fs::rename(&again, &tree).map_err(|e| io("moving", &again, &e))?;
            }
        } else {
            // One file: the program itself.
            let target = tree.join(file_name_of(&asset.name));
            fs::copy(&fetched.path, &target).map_err(|e| io("copying to", &target, &e))?;
        }
        if cancel.is_cancelled() {
            return Err(InstallError::Cancelled);
        }

        on_step(Step { stage: Stage::Finishing, done: 0, total: None, rate: None });
        layout::hoist_wrapper(&tree, request.platform)?;
        layout::merge_into(&tree, folder)?;
        if format == Format::AppImage {
            remove_stale_appimages(folder, &file_name_of(&asset.name));
        }
        programs::make_runnable(folder, request.platform);

        let found = programs::find(folder, true, request.platform);
        if found.programs.is_empty() {
            return Err(InstallError::NoProgram);
        }
        fs::write(folder.join(VERSION_FILE), tag).map_err(|e| io("writing", &folder.join(VERSION_FILE), &e))?;
        if let Err(error) = fs::remove_file(folder.join(INCOMPLETE_FILE))
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(io("removing", &folder.join(INCOMPLETE_FILE), &error));
        }
        Ok(Installed {
            folder: folder.clone(),
            version: tag.to_string(),
            asset: asset.name.clone(),
            program: found.programs.first().cloned(),
            needs_runner: found.needs_runner,
            notes: resolved.release.notes.clone(),
            page: resolved.release.page.clone(),
        })
    }

    fn unpack(
        &self,
        format: Format,
        archive: &Path,
        into: &Path,
        scratch: &Path,
        cancel: &Cancel,
        on_step: &mut dyn FnMut(Step),
    ) -> Result<(), InstallError> {
        let stats = archive::extract(format, archive, into, scratch, self.limits, cancel, &mut |p| {
            on_step(Step { stage: Stage::Extracting, done: p.done, total: p.total, rate: None });
        })?;
        tracing::debug!(entries = stats.entries, bytes = stats.bytes, skipped = stats.skipped, "unpacked");
        Ok(())
    }
}

fn describe(format: Format) -> String {
    match format {
        Format::Unsupported(kind) => kind.to_string(),
        other => format!("{other:?}"),
    }
}

/// What a downloaded file is. The name decides, except for a name that says nothing (a package link on GitLab): then the
/// first bytes do, and a file that is not an archive is a program in one file.
fn detect_format(name: &str, path: &Path) -> Result<Format, InstallError> {
    let named = Format::of_name(name);
    if named != Format::Bare {
        return Ok(named);
    }
    let mut head = [0u8; 8];
    let read = std::fs::File::open(path).and_then(|mut f| std::io::Read::read(&mut f, &mut head)).map_err(|e| io("reading", path, &e))?;
    Ok(Format::sniff(&head[..read]).unwrap_or(Format::Bare))
}

/// The one file of a tree when it is a `.tar.gz` and nothing else (launcher files aside).
fn lone_tar_gz(tree: &Path) -> Option<PathBuf> {
    let mut only: Option<PathBuf> = None;
    let mut pending = vec![tree.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).ok()?.flatten() {
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).ok()?;
            if meta.is_dir() {
                pending.push(path);
            } else if !path.file_name().and_then(|n| n.to_str()).is_some_and(layout::is_metadata_file) {
                if only.is_some() {
                    return None;
                }
                only = Some(path);
            }
        }
    }
    only.filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.to_ascii_lowercase().ends_with(".tar.gz")))
}

/// A new AppImage is named for its version; the old ones in the folder's top level would pile up.
fn remove_stale_appimages(folder: &Path, keep: &str) {
    let Ok(entries) = fs::read_dir(folder) else { return };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name != keep
            && name.to_ascii_lowercase().ends_with(".appimage")
            && entry.path().is_file()
            && let Err(error) = fs::remove_file(entry.path())
        {
            tracing::warn!(file = %name, %error, "an old AppImage could not be removed");
        }
    }
}

#[cfg(test)]
mod tests;
