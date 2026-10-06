//! A Thunderstore mod names the packages it needs (`Owner-Name-1.2.3`); they are installed first, each once, newest version
//! (Thunderstore's own tools take the newest too). One already installed in any version is left as it is.
use std::collections::HashSet;

use reclaw_net::Cancel;

use super::{ModInstaller, Step, Target};
use crate::{
    client::ModSites,
    error::ModError,
    package::{Download, Package},
    sidecar::{Document, Record},
    source::Provider,
    thunderstore,
};

/// How deep a chain of dependencies is followed before it is taken to be a loop the visited set missed.
const MAX_DEPTH: usize = 16;

/// Install `package` and, first, whatever it needs that is not installed. Returns the records written, dependencies first.
pub fn install_with_dependencies(
    sites: &ModSites,
    installer: &ModInstaller,
    target: &Target,
    package: &Package,
    cancel: &Cancel,
    on_step: &mut dyn FnMut(Step),
) -> Result<Vec<Record>, ModError> {
    let mut work = Work { sites, installer, target, cancel, visited: HashSet::new(), done: Vec::new() };
    work.install(package, None, 0, on_step)?;
    Ok(work.done)
}

struct Work<'a> {
    sites: &'a ModSites,
    installer: &'a ModInstaller,
    target: &'a Target,
    cancel: &'a Cancel,
    visited: HashSet<String>,
    done: Vec<Record>,
}

impl Work<'_> {
    fn install(&mut self, package: &Package, known: Option<Download>, depth: usize, on_step: &mut dyn FnMut(Step)) -> Result<(), ModError> {
        if !self.visited.insert(package.id.to_ascii_lowercase()) {
            return Ok(());
        }
        if self.cancel.is_cancelled() {
            return Err(ModError::Cancelled);
        }
        let download = match known {
            Some(download) => download,
            None => self.sites.download_for(package)?,
        };
        if package.provider == Provider::Thunderstore && depth < MAX_DEPTH {
            for dependency in &download.dependencies {
                let Some((full_name, _)) = thunderstore::parse_dependency(dependency) else {
                    tracing::debug!(dependency, "a dependency Reclaw cannot read was skipped");
                    continue;
                };
                if Document::load(&self.target.folder)?.has_full_name(Provider::Thunderstore, &package.source_key, &full_name) {
                    continue;
                }
                let Some((owner, name)) = thunderstore::split_full_name(&full_name) else { continue };
                let (needed, needed_download) = self.sites.thunderstore_package(&package.source_key, owner, name)?;
                tracing::info!(mod_id = %package.id, dependency = %full_name, "installing a dependency first");
                self.install(&needed, Some(needed_download), depth + 1, on_step)?;
            }
        }
        let record = self.installer.install(self.target, package, &download, self.cancel, on_step)?;
        self.done.push(record);
        Ok(())
    }
}
