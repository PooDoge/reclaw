//! Loading the community catalog: the index, then every list it names and the platform metadata, in parallel, each through the
//! shared client with its answer saved to disk. A part that cannot be loaded does not spoil the rest: the saved copy stands in
//! for it, or it is left out, and what happened is recorded as a [`Problem`].
use std::{thread, time::Duration};

use reclaw_catalog::{CommunityIndex, IndexSource, PlatformDocument, PlatformIndex, parse_list, timestamp::Timestamp};
use reclaw_net::{Fetched, Net, NetError, Request, cache::unix_now};

use crate::{
    error::SyncError,
    snapshot::{CatalogSnapshot, ListSnapshot, Origin, PlatformSnapshot, Problem},
};

/// Where the community catalog lives: the index of the lists Quiver's users share.
pub const DEFAULT_INDEX_URL: &str = "https://raw.githubusercontent.com/tgeorgiadis/quiver-community-app-catalog/main/index.json";

/// A saved copy younger than this is used without asking. The host serves these files with a five-minute lifetime of its own.
pub const DEFAULT_TTL: Duration = Duration::from_secs(300);

const INDEX_LIMIT: u64 = 1024 * 1024;
const LIST_LIMIT: u64 = 16 * 1024 * 1024;
const PLATFORM_LIMIT: u64 = reclaw_catalog::platform_index::MAX_BYTES as u64;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Online,
    SavedOnly,
}

#[derive(Clone)]
pub struct CatalogSync {
    net: Net,
    index_url: String,
    ttl: Duration,
}

impl CatalogSync {
    pub fn new(net: Net) -> Self {
        Self { net, index_url: DEFAULT_INDEX_URL.to_string(), ttl: DEFAULT_TTL }
    }

    pub fn with_index_url(mut self, url: impl Into<String>) -> Self {
        self.index_url = url.into();
        self
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Ask the network (or the saved copies that are fresh enough) and build the snapshot. Call from a worker thread.
    pub fn refresh(&self) -> Result<CatalogSnapshot, SyncError> {
        self.load(Mode::Online)
    }

    /// Whatever is already on disk, with no network at all: what the app shows while the first refresh is still running, and
    /// all it has when offline. `None` if nothing usable was ever saved.
    pub fn saved(&self) -> Option<CatalogSnapshot> {
        self.load(Mode::SavedOnly).ok()
    }

    fn get(&self, mode: Mode, url: &str, max_bytes: u64) -> Result<Fetched, NetError> {
        let request = Request::get(url).max_bytes(max_bytes).cached(self.ttl, true);
        match mode {
            Mode::Online => self.net.fetch(&request),
            Mode::SavedOnly => self.net.cached(&request).ok_or_else(|| NetError::Other("nothing was saved".into())),
        }
    }

    fn load(&self, mode: Mode) -> Result<CatalogSnapshot, SyncError> {
        let started = std::time::Instant::now();
        let online = mode == Mode::Online;
        if online {
            tracing::info!(index = %self.index_url, "refreshing the catalog");
        }
        let result = self.load_parts(mode);
        let ms = started.elapsed().as_millis() as u64;
        match &result {
            Ok(snapshot) => {
                // A refresh is news; reading the saved copy at start is routine.
                let (apps, lists, problems) = (snapshot.apps().len(), snapshot.lists.len(), snapshot.problems.len());
                if online {
                    tracing::info!(apps, lists, problems, platform = snapshot.platform.is_some(), ms, "catalog loaded");
                    for problem in &snapshot.problems {
                        tracing::warn!(what = %problem.what, detail = %problem.detail, hint = problem.hint.as_deref(), "catalog problem");
                    }
                } else {
                    tracing::debug!(apps, lists, problems, ms, "saved catalog read");
                }
            }
            Err(error) if online => tracing::error!(%error, hint = error.hint(), ms, "the catalog could not be loaded"),
            Err(error) => tracing::debug!(%error, "no saved catalog"),
        }
        result
    }

    fn load_parts(&self, mode: Mode) -> Result<CatalogSnapshot, SyncError> {
        let mut problems = Vec::new();
        let fetched = self.get(mode, &self.index_url, INDEX_LIMIT).map_err(SyncError::NoIndex)?;
        let index = CommunityIndex::parse(&fetched.text()).map_err(SyncError::BadIndex)?;
        note_stale(&mut problems, "The community catalog index", &fetched);
        let (index_origin, mut oldest) = (Origin::from(fetched.source), fetched.fetched_at);

        let sources = index.sources();
        // One thread per list: a handful of small requests that the client multiplexes over one connection.
        let answers: Vec<(IndexSource, Result<Fetched, NetError>)> = thread::scope(|scope| {
            // Named, so a line in the log says which list's request it was.
            let workers: Vec<_> = sources
                .iter()
                .enumerate()
                .map(|(n, source)| {
                    thread::Builder::new()
                        .name(format!("reclaw-list-{n}"))
                        .spawn_scoped(scope, move || self.get(mode, &source.url, LIST_LIMIT))
                })
                .collect();
            sources
                .iter()
                .cloned()
                .zip(workers)
                .map(|(source, worker)| {
                    let answer = match worker {
                        Ok(worker) => {
                            worker.join().unwrap_or_else(|_| Err(NetError::Other("a download worker stopped unexpectedly".into())))
                        }
                        Err(error) => Err(NetError::Other(format!("a download worker could not start: {error}"))),
                    };
                    (source, answer)
                })
                .collect()
        });
        let mut lists = Vec::new();
        for (source, answer) in answers {
            let label = if source.name.is_empty() { source.id.clone() } else { source.name.clone() };
            match answer {
                Err(error) => problems.push(problem(&format!("The list {label}"), &error)),
                Ok(fetched) => match parse_list(&fetched.text()) {
                    Err(error) => problems.push(Problem {
                        what: format!("The list {label}"),
                        detail: format!("could not be read: {error}"),
                        hint: None,
                    }),
                    Ok(list) => {
                        note_stale(&mut problems, &format!("The list {label}"), &fetched);
                        if let Some(first) = list.skipped.first() {
                            problems.push(Problem {
                                what: format!("The list {label}"),
                                detail: format!(
                                    "has {} entries that could not be read and were skipped (the first: {})",
                                    list.skipped.len(),
                                    first.reason
                                ),
                                hint: None,
                            });
                        }
                        oldest = oldest.min(fetched.fetched_at);
                        lists.push(ListSnapshot { source, list, origin: fetched.source.into(), fetched_at: fetched.fetched_at });
                    }
                },
            }
        }

        let platform = index.platform_metadata_url.as_deref().and_then(|url| self.load_platform(mode, url, &mut problems));
        if let Some(platform) = &platform {
            oldest = oldest.min(platform.fetched_at);
        }
        Ok(CatalogSnapshot { index, index_origin, lists, platform, problems, fetched_at: oldest })
    }

    fn load_platform(&self, mode: Mode, url: &str, problems: &mut Vec<Problem>) -> Option<PlatformSnapshot> {
        let what = "The platform metadata";
        let fetched = match self.get(mode, url, PLATFORM_LIMIT) {
            Ok(fetched) => fetched,
            Err(error) => {
                problems.push(problem(what, &error));
                return None;
            }
        };
        let document = match PlatformDocument::parse(&fetched.text()) {
            Ok(document) => document,
            Err(error) => {
                problems.push(Problem { what: what.into(), detail: format!("is not valid and was not used: {error}"), hint: None });
                return None;
            }
        };
        if Timestamp::now().is_some_and(|now| document.is_from_the_future(now)) {
            problems.push(Problem {
                what: what.into(),
                detail: "claims to be from the future, so it was not used (is this computer's clock right?)".into(),
                hint: None,
            });
            return None;
        }
        note_stale(problems, what, &fetched);
        let index = PlatformIndex::from_documents([&document]);
        Some(PlatformSnapshot { document, index, origin: fetched.source.into(), fetched_at: fetched.fetched_at })
    }
}

fn problem(what: &str, error: &NetError) -> Problem {
    Problem { what: what.to_string(), detail: format!("could not be loaded: {error}"), hint: error.hint().map(str::to_string) }
}

/// A saved copy that stands in for a failed request is worth saying so once.
fn note_stale(problems: &mut Vec<Problem>, what: &str, fetched: &Fetched) {
    if let Some(reason) = &fetched.stale_because {
        let minutes = unix_now().saturating_sub(fetched.fetched_at) / 60;
        problems.push(Problem {
            what: what.to_string(),
            detail: format!(
                "could not be refreshed ({reason}); showing a saved copy from {}",
                if minutes < 90 { format!("{minutes} minutes ago") } else { format!("{} hours ago", minutes / 60) }
            ),
            hint: reason.hint().map(str::to_string),
        });
    }
}

#[cfg(test)]
mod tests;
