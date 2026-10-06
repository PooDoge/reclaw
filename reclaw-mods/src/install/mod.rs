//! Installing and removing one mod in one app's folder.
//!
//! An install downloads the file (resumable, through `reclaw-net`), unpacks it with `reclaw-install`'s extractor (the same
//! rules that keep a hostile archive inside its folder), decides where each file goes (`plan`), refuses to replace another mod's
//! files, moves the files in with the replaced ones set aside, writes the record, and only then removes what an earlier version
//! of the same mod left that the new one does not have. A failure before the record is written puts the folder back as it was.
//!
//! * this file: [`Target`], [`ModInstaller`], install and remove
//! * `files`: paths kept inside the folder, moving with undo, removing and pruning
//! * `deps`: installing a Thunderstore mod's dependencies first
use std::{
    fs,
    path::{Path, PathBuf},
};

use reclaw_catalog::mods::ModsConfig;
use reclaw_install::{
    Format,
    archive::{self, Limits},
};
use reclaw_net::{Cancel, DownloadRequest, Net};

use crate::{
    error::ModError,
    package::{Download, Package},
    plan::{self, Rules},
    sidecar::{Document, Record},
    source::Provider,
    thunderstore,
};

mod deps;
pub mod files;

pub use deps::install_with_dependencies;

/// Where an app's mods go.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Target {
    /// The app's own folder.
    pub folder: PathBuf,
    pub config: ModsConfig,
}

impl Target {
    /// The mods folder: the configured path inside the app's folder (the catalog has already refused `..` and the like).
    pub fn mods_dir(&self) -> Result<PathBuf, ModError> {
        let path = reclaw_catalog::mods::normalize_path(&self.config.path);
        if path.is_empty() {
            return Err(ModError::NoModsFolder);
        }
        Ok(path.split('/').fold(self.folder.clone(), |dir, part| dir.join(part)))
    }
}

/// Where the work is, for the screens.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stage {
    Downloading,
    Unpacking,
    Placing,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Step {
    pub stage: Stage,
    pub done: u64,
    pub total: Option<u64>,
    /// Bytes a second, while downloading.
    pub rate: Option<u64>,
    /// The mod being worked on (a dependency's name while it is installed first).
    pub name: String,
}

/// Where a mod is unpacked before it is moved into place: inside the app's folder (so a move never crosses disks) and outside
/// the mods folder (so the game never sees it).
pub const STAGE_DIR: &str = ".reclaw-mod-stage";

#[derive(Clone)]
pub struct ModInstaller {
    net: Net,
    /// Where downloads wait until they are installed; a cut-off one continues from here.
    downloads: PathBuf,
    limits: Limits,
}

/// FNV-1a: steady across Rust releases, so a cut-off download is found again after an update.
fn fnv(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3))
}

/// How a downloaded file becomes files in the mods folder.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Archive(Format),
    /// A mod in one file (`x.nrm`, `x.o2r`): placed as it is. A `.nrm` is a zip inside, so its name decides, not its bytes.
    Single,
}

fn kind_of(name: &str, path: &Path) -> Result<Kind, ModError> {
    match Format::of_name(name) {
        format if format.is_archive() => Ok(Kind::Archive(format)),
        // No extension: the first bytes decide.
        Format::Bare => {
            let mut head = [0u8; 8];
            let read = fs::File::open(path)
                .and_then(|mut f| std::io::Read::read(&mut f, &mut head))
                .map_err(|e| ModError::io(format!("reading {}", path.display()), &e))?;
            match Format::sniff(&head[..read]) {
                Some(format) if format.is_archive() => Ok(Kind::Archive(format)),
                _ => Err(ModError::NoDownload(format!("{name} is neither an archive nor a file with a known kind"))),
            }
        }
        _ => Ok(Kind::Single),
    }
}

/// A name that is one path component, safe to make a file of.
fn file_name_of(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .chars()
        .map(|c| if c.is_control() || c == ':' { '_' } else { c })
        .collect();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." { "download".to_string() } else { cleaned }
}

impl ModInstaller {
    pub fn new(net: Net, downloads: PathBuf) -> Self {
        // A mod is far smaller than a game; an archive that unpacks to more than this is not a mod.
        Self { net, downloads, limits: Limits { max_bytes: 16 * 1024 * 1024 * 1024, max_entries: 200_000 } }
    }

    /// Install `download` of `package` into `target`, replacing an earlier version of the same mod. Its dependencies are not
    /// looked at; [`install_with_dependencies`] does that.
    pub fn install(
        &self,
        target: &Target,
        package: &Package,
        download: &Download,
        cancel: &Cancel,
        on_step: &mut dyn FnMut(Step),
    ) -> Result<Record, ModError> {
        let mods_dir = target.mods_dir()?;
        // Read first: a record that cannot be read stops the install before anything is fetched or changed.
        Document::load(&target.folder)?;
        let name =
            file_name_of(download.file_name.as_deref().unwrap_or_else(|| download.url.rsplit('/').find(|s| !s.is_empty()).unwrap_or("")));
        let fetched = self.downloads.join(format!("{:016x}", fnv(&download.url))).join(&name);
        tracing::info!(mod_id = %package.id, version = %download.version, folder = %mods_dir.display(), "installing a mod");

        let mut request = DownloadRequest::new(&download.url, &fetched);
        if let Some(size) = download.size.filter(|s| *s > 0) {
            request = request.size(size);
        }
        let step = |stage, done, total, rate| Step { stage, done, total, rate, name: package.name.clone() };
        self.net
            .download(&request, cancel, &mut |p| on_step(step(Stage::Downloading, p.downloaded, p.total, Some(p.bytes_per_sec as u64))))?;

        let stage = target.folder.join(STAGE_DIR).join(format!("{:016x}", fnv(&format!("{}/{}", package.provider.id(), package.id))));
        let result = self.place(target, &mods_dir, package, download, &fetched, &name, &stage, cancel, on_step);
        if stage.exists()
            && let Err(error) = fs::remove_dir_all(&stage)
        {
            tracing::warn!(path = %stage.display(), %error, "a mod's unpacked files could not be cleared away");
        }
        if let Some(parent) = stage.parent() {
            // Only when empty: another mod of this app may be installing beside it.
            let _ = fs::remove_dir(parent);
        }
        match &result {
            Ok(record) => {
                if let Some(dir) = fetched.parent()
                    && let Err(error) = fs::remove_dir_all(dir)
                {
                    tracing::debug!(path = %dir.display(), %error, "a mod's download could not be cleared away");
                }
                tracing::info!(mod_id = %package.id, version = %record.version, files = record.files.len(), "mod installed");
            }
            Err(error) if error.is_cancelled() => tracing::info!(mod_id = %package.id, "the mod install was cancelled"),
            Err(error) => tracing::warn!(mod_id = %package.id, %error, "the mod install failed"),
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn place(
        &self,
        target: &Target,
        mods_dir: &Path,
        package: &Package,
        download: &Download,
        fetched: &Path,
        name: &str,
        stage: &Path,
        cancel: &Cancel,
        on_step: &mut dyn FnMut(Step),
    ) -> Result<Record, ModError> {
        let tree = stage.join("tree");
        if stage.exists() {
            fs::remove_dir_all(stage).map_err(|e| ModError::io(format!("clearing {}", stage.display()), &e))?;
        }
        fs::create_dir_all(&tree).map_err(|e| ModError::io(format!("making {}", tree.display()), &e))?;
        match kind_of(name, fetched)? {
            Kind::Archive(format) => {
                archive::extract(format, fetched, &tree, stage, self.limits, cancel, &mut |p| {
                    on_step(Step { stage: Stage::Unpacking, done: p.done, total: p.total, rate: None, name: package.name.clone() });
                })?;
            }
            Kind::Single => {
                fs::copy(fetched, tree.join(name)).map_err(|e| ModError::io(format!("copying {name}"), &e))?;
            }
        }
        if cancel.is_cancelled() {
            return Err(ModError::Cancelled);
        }
        on_step(Step { stage: Stage::Placing, done: 0, total: None, rate: None, name: package.name.clone() });

        let metadata: &[&str] = if package.provider == Provider::Thunderstore { &thunderstore::METADATA_FILES } else { &[] };
        let folder_name = mods_dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let placements = plan::placements(
            &files::regular_files(&tree)?,
            Rules { metadata, layout: target.config.layout, mods_folder_name: folder_name, folder_name: &package.name },
        );
        if placements.is_empty() {
            return Err(ModError::Empty);
        }

        let mut document = Document::load(&target.folder)?;
        let replacing = |r: &Record| r.is(package.provider, &package.id);
        if let Some((owner, clash)) = plan::conflicts(&placements, &document, &replacing).into_iter().next() {
            return Err(ModError::Conflict { name: package.name.clone(), owner, files: clash });
        }

        fs::create_dir_all(mods_dir).map_err(|e| ModError::io(format!("making {}", mods_dir.display()), &e))?;
        let mut moves = files::Moves::new(stage.join("replaced"));
        for placement in &placements {
            let destination = files::inside(mods_dir, &placement.to)
                .ok_or_else(|| ModError::BadAnswer(format!("{} would be placed outside the mods folder", placement.to)));
            let moved = destination.and_then(|to| moves.place(&tree.join(&placement.from), &to));
            if let Err(error) = moved {
                moves.undo(mods_dir);
                return Err(error);
            }
        }

        let record = Record {
            provider: package.provider.id().to_string(),
            source_key: package.source_key.clone(),
            id: package.id.clone(),
            full_name: package.full_name.clone(),
            owner: package.owner.clone(),
            name: package.name.clone(),
            version: download.version.clone(),
            download_file_id: download.file_id.clone(),
            download_file_name: download.file_name.clone().filter(|_| download.file_id.is_some()),
            files: placements.iter().map(|p| p.to.clone()).collect(),
        };
        let earlier: Vec<String> = document.mods.iter().filter(|r| replacing(r)).flat_map(|r| r.files.clone()).collect();
        document.mods.retain(|r| !replacing(r));
        document.mods.push(record.clone());
        if let Err(error) = document.save(&target.folder) {
            moves.undo(mods_dir);
            return Err(error);
        }
        moves.keep();
        // What the earlier version had and this one does not.
        for old in earlier.iter().filter(|old| !record.files.iter().any(|f| f.eq_ignore_ascii_case(old))) {
            if let Err(error) = files::remove(mods_dir, old) {
                tracing::warn!(file = %old, %error, "a file of the earlier version of a mod could not be removed");
            }
        }
        Ok(record)
    }

    /// Remove every installed copy of the mod: its files, the folders that leaves empty, and its record. `Ok(false)` when it was
    /// not installed. A file that cannot be removed stays in the record, so a later try still knows it.
    pub fn uninstall(&self, target: &Target, provider: Provider, id: &str) -> Result<bool, ModError> {
        uninstall(target, provider, id)
    }
}

/// [`ModInstaller::uninstall`], which needs no network.
pub fn uninstall(target: &Target, provider: Provider, id: &str) -> Result<bool, ModError> {
    let mods_dir = target.mods_dir()?;
    let mut document = Document::load(&target.folder)?;
    if document.find(provider, id).next().is_none() {
        return Ok(false);
    }
    let mut failures = Vec::new();
    for record in document.mods.iter_mut().filter(|r| r.is(provider, id)) {
        record.files.retain(|file| match files::remove(&mods_dir, file) {
            Ok(()) => false,
            Err(error) => {
                failures.push(error);
                true
            }
        });
    }
    document.mods.retain(|r| !(r.is(provider, id) && r.files.is_empty()));
    document.save(&target.folder)?;
    tracing::info!(provider = provider.id(), mod_id = id, left = failures.len(), "mod removed");
    match failures.into_iter().next() {
        Some(error) => Err(error),
        None => Ok(true),
    }
}

#[cfg(test)]
mod tests;
