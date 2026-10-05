//! Which of a release's files to install on this machine. Quiver's `DownloadAssetPolicy`, with one change that matters for a
//! launcher driven by a game pad: where Quiver asks the person whenever more than one file qualifies, Reclaw takes the one
//! build made for this platform and asks only when it still cannot tell.
use crate::{
    format::Format,
    matcher::{is_dedicated_device_asset, is_ios_asset, is_windows_asset, matches_platform},
    platform::Platform,
    release::{Asset, Release},
};

/// The files of one release, sorted into what fits this machine.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Selection {
    /// Made for this platform (native builds first), or a Windows build on Linux, which a compatibility layer can run.
    pub eligible: Vec<Asset>,
    /// Files that say nothing about their platform: the person may know better than the name does.
    pub uncertain: Vec<Asset>,
    /// Why `eligible` is empty, in words for the person.
    pub reason: Option<String>,
}

/// Files whose name gives no platform at all.
fn has_known_platform(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    is_ios_asset(name)
        || is_dedicated_device_asset(name)
        || is_windows_asset(name)
        || matches_platform(name, "macOS")
        || ["linux", "appimage", "flatpak", "android", "arm64-v8a", "switch"].iter().any(|w| lower.contains(w))
        || [".apk", ".deb", ".rpm", ".tar.gz", ".tar.xz"].iter().any(|e| lower.ends_with(e))
}

impl Selection {
    /// Sort `release`'s files for `platform`. `filter` is the catalog's `releaseAssetFilter`: a case-insensitive piece of a name.
    pub fn of(release: &Release, platform: Platform, filter: Option<&str>) -> Self {
        let id = platform.identifier();
        let all = release.downloadable(None);
        let filtered = release.downloadable(filter);

        let native = |a: &&Asset| matches_platform(&a.name, id);
        let fits = |a: &&Asset| native(a) || (platform.is_linux() && is_windows_asset(&a.name));
        let mut fitting: Vec<&Asset> = filtered.iter().copied().filter(fits).collect();
        fitting.sort_by_key(|a| !native(a)); // stable: native first, the release's own order within each group
        let installable: Vec<&Asset> = fitting.iter().copied().filter(|a| Format::of_name(&a.name).is_installable()).collect();
        let uncertain: Vec<Asset> = filtered
            .iter()
            .copied()
            .filter(|a| !has_known_platform(&a.name) && Format::of_name(&a.name).is_installable())
            .cloned()
            .collect();

        let reason = if !installable.is_empty() {
            None
        } else if all.is_empty() {
            Some("This release has no installable download files.".to_string())
        } else if filtered.is_empty() {
            Some(format!("No download files match the release asset filter \"{}\".", filter.unwrap_or_default().trim()))
        } else if let Some(kind) = fitting.iter().find_map(|a| match Format::of_name(&a.name) {
            Format::Unsupported(kind) => Some(kind),
            _ => None,
        }) {
            Some(format!("This release's download for {id} is {kind}, which Reclaw does not install."))
        } else {
            Some(format!("This release has no recognized download for {id}."))
        };
        Self { eligible: installable.into_iter().cloned().collect(), uncertain, reason }
    }

    /// The one file to install without asking: the only eligible file; failing that the only one made for this platform
    /// (the Windows builds on Linux are then the fallback, not a rival); failing that the only one left once the filter has
    /// narrowed the list. `None` means someone has to choose (or nothing fits).
    pub fn automatic(&self, platform: Platform) -> Option<&Asset> {
        match self.eligible.as_slice() {
            [only] => Some(only),
            many if many.len() > 1 => {
                let mut native = many.iter().filter(|a| matches_platform(&a.name, platform.identifier()));
                match (native.next(), native.next()) {
                    (Some(first), None) => Some(first),
                    _ => None,
                }
            }
            _ => None,
        }
    }

    /// Several files qualify and none is clearly the one, or only files of unknown platform exist.
    pub fn needs_choice(&self, platform: Platform) -> bool {
        self.automatic(platform).is_none() && (self.eligible.len() > 1 || (self.eligible.is_empty() && !self.uncertain.is_empty()))
    }

    /// What to offer when asking: the eligible files, or the uncertain ones when nothing is eligible.
    pub fn choices(&self) -> &[Asset] {
        if self.eligible.is_empty() { &self.uncertain } else { &self.eligible }
    }
}
