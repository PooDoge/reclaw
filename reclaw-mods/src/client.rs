//! Asking the two sites, through the program's one HTTP client (`reclaw-net`: its user agent, retries, rate-limit care and disk
//! cache). A listing is kept for a while and reused, and a saved one is shown when the network fails; what an install takes is
//! always asked again, so an install gets the version that is current now.
use std::time::Duration;

use reclaw_net::{Net, NetError, Request};

use crate::{
    error::ModError,
    gamebanana,
    package::{Download, Package, Page, Sort},
    source::{Provider, Source},
    thunderstore,
};

/// Where the two sites' APIs are. The real addresses unless a test points them at a server of its own.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Endpoints {
    pub thunderstore: String,
    pub gamebanana: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self { thunderstore: "https://thunderstore.io".to_string(), gamebanana: "https://gamebanana.com".to_string() }
    }
}

/// How long a listing is used without asking again.
const LISTING_FRESH: Duration = Duration::from_secs(15 * 60);
/// A community's sections change about never.
const SECTIONS_FRESH: Duration = Duration::from_secs(12 * 60 * 60);
/// A listing page is small; a hostile answer may not be large.
const MAX_ANSWER: u64 = 8 * 1024 * 1024;

#[derive(Clone)]
pub struct ModSites {
    net: Net,
    endpoints: Endpoints,
}

impl ModSites {
    pub fn new(net: Net) -> Self {
        Self { net, endpoints: Endpoints::default() }
    }

    pub fn with_endpoints(mut self, endpoints: Endpoints) -> Self {
        self.endpoints = endpoints;
        self
    }

    pub fn net(&self) -> &Net {
        &self.net
    }

    fn get(&self, url: String, fresh_for: Duration, stale_ok: bool) -> Result<reclaw_net::Fetched, ModError> {
        let request = Request::get(url).accept("application/json").max_bytes(MAX_ANSWER).cached(fresh_for, stale_ok);
        Ok(self.net.fetch(&request)?)
    }

    /// One page of a source's mods. `fresh`: the person asked, so a saved page is used only if the site confirms it.
    pub fn list(&self, source: &Source, page: u32, sort: Sort, fresh: bool) -> Result<Page, ModError> {
        let ttl = if fresh { Duration::ZERO } else { LISTING_FRESH };
        let page = match source.provider {
            Provider::Thunderstore => {
                let section = self.mods_section(&source.key);
                let url = thunderstore::listing_url(&self.endpoints.thunderstore, &source.key, page, sort, section.as_deref());
                thunderstore::parse_listing(&self.get(url, ttl, true)?.body, &source.key)
            }
            Provider::GameBanana => {
                let url = gamebanana::index_url(&self.endpoints.gamebanana, &source.key, page, sort);
                gamebanana::parse_index(&self.get(url, ttl, true)?.body, &source.key, page)
            }
        }
        .map_err(ModError::BadAnswer)?;
        tracing::debug!(provider = source.provider.id(), source = %source.key, mods = page.packages.len(), next = ?page.next, "mods listed");
        Ok(page)
    }

    /// The first pages of a source, up to `limit` mods.
    pub fn list_up_to(&self, source: &Source, limit: usize, sort: Sort, fresh: bool) -> Result<Vec<Package>, ModError> {
        let mut found: Vec<Package> = Vec::new();
        let mut next = Some(1);
        while let Some(page) = next {
            let listed = self.list(source, page, sort, fresh)?;
            found.extend(listed.packages.into_iter().filter(|p| !found.iter().any(|f| f.id == p.id)).collect::<Vec<_>>());
            next = listed.next.filter(|n| *n > page && found.len() < limit);
        }
        found.truncate(limit);
        Ok(found)
    }

    /// The community's "Mods" section, so modpacks and tools stay out of the list. `None` (all packages) when the community has no
    /// such section or the question fails: a longer list is better than none.
    fn mods_section(&self, community: &str) -> Option<String> {
        match self.get(thunderstore::filters_url(&self.endpoints.thunderstore, community), SECTIONS_FRESH, true) {
            Ok(answer) => thunderstore::parse_mods_section(&answer.body),
            Err(error) => {
                tracing::debug!(community, %error, "the community's sections could not be read; listing every package");
                None
            }
        }
    }

    /// What installing `package` downloads now: the newest version (Thunderstore) or the mod's first archive (GameBanana).
    pub fn download_for(&self, package: &Package) -> Result<Download, ModError> {
        match package.provider {
            Provider::Thunderstore => {
                let (owner, name) = thunderstore::split_full_name(&package.id)
                    .ok_or_else(|| ModError::BadAnswer(format!("\"{}\" is not a Thunderstore package name", package.id)))?;
                Ok(self.thunderstore_package(&package.source_key, owner, name)?.1)
            }
            Provider::GameBanana => {
                let url = gamebanana::detail_url(&self.endpoints.gamebanana, &package.id);
                let detail = gamebanana::parse_detail(&self.get(url, Duration::ZERO, false)?.body).map_err(ModError::BadAnswer)?;
                gamebanana::choose_download(&detail)
                    .ok_or_else(|| ModError::NoDownload(format!("{} offers no file to download", package.name)))
            }
        }
    }

    /// A Thunderstore package by owner and name: how a dependency is found.
    pub fn thunderstore_package(&self, community: &str, owner: &str, name: &str) -> Result<(Package, Download), ModError> {
        let url = thunderstore::package_url(&self.endpoints.thunderstore, owner, name);
        let answer = self.get(url, Duration::ZERO, false).map_err(|error| match error {
            ModError::Net(NetError::Status { status: 404, .. }) => ModError::NoDownload(format!("{owner}-{name} is not on Thunderstore")),
            other => other,
        })?;
        thunderstore::parse_package(&answer.body, community).map_err(ModError::BadAnswer)
    }
}
