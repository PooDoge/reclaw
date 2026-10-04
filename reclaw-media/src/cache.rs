//! [`MediaCache`]: given an address, give back a file on disk, from the store when it is fresh and
//! from the network when it is not. It also decides what is worth keeping and what failures to
//! remember, so a README with a dead image link does not retry it on every redraw.
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, SystemTime},
};

use crate::{
    dimensions::{HEAD_BYTES, dimensions},
    fetch::{Fetch, FetchError},
    sniff::{looks_like_text, sniff_image},
    source::{MediaUrl, UrlError},
    store::{DiskStore, Entry, Kind},
};

/// What the caller wants the address to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Want {
    Image,
    Text,
}

impl Want {
    fn matches(self, kind: Kind) -> bool {
        matches!((self, kind), (Self::Image, Kind::Image(_)) | (Self::Text, Kind::Text))
    }
}

/// The limits and lifetimes. The defaults suit a launcher; tests shrink them.
#[derive(Debug, Clone)]
pub struct Policy {
    pub max_image_bytes: u64,
    pub max_text_bytes: u64,
    /// A file this young is used without asking the network.
    pub fresh_for: Duration,
    /// A file older than `fresh_for` is fetched again, but kept as the fallback for this long.
    pub stale_ok_for: Duration,
    /// The cache is trimmed to this size.
    pub max_total_bytes: u64,
    /// After a failure the address is not tried again for this long.
    pub failure_memory: Duration,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            max_image_bytes: 8 * 1024 * 1024,
            max_text_bytes: 1024 * 1024,
            fresh_for: Duration::from_secs(24 * 60 * 60),
            stale_ok_for: Duration::from_secs(30 * 24 * 60 * 60),
            max_total_bytes: 256 * 1024 * 1024,
            failure_memory: Duration::from_secs(10 * 60),
        }
    }
}

/// A file ready to be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cached {
    pub path: PathBuf,
    pub kind: Kind,
    /// Older than it should be: the network failed and this is what there was.
    pub stale: bool,
    /// A picture's size in pixels, when its header could be read.
    pub size: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MediaError {
    #[error("{0}")]
    Blocked(UrlError),
    #[error("{0}")]
    Fetch(FetchError),
    #[error("not an image Reclaw can show")]
    NotAnImage,
    #[error("not text")]
    NotText,
    #[error("could not be saved: {0}")]
    Disk(String),
    /// Failed a moment ago; not tried again yet.
    #[error("failed recently: {0}")]
    Remembered(String),
}

pub struct MediaCache {
    store: DiskStore,
    fetcher: Arc<dyn Fetch>,
    policy: Policy,
    clock: Arc<dyn Fn() -> SystemTime + Send + Sync>,
    failures: Mutex<HashMap<(String, Want), (SystemTime, String)>>,
    stores: AtomicUsize,
}

/// Trim the cache after this many new files, not after each: walking the folders costs more than a write.
const TRIM_EVERY: usize = 16;

impl MediaCache {
    pub fn new(store: DiskStore, fetcher: Arc<dyn Fetch>, policy: Policy) -> Self {
        Self::with_clock(store, fetcher, policy, Arc::new(SystemTime::now))
    }

    /// The clock is a parameter so tests can move time without waiting.
    pub fn with_clock(store: DiskStore, fetcher: Arc<dyn Fetch>, policy: Policy, clock: Arc<dyn Fn() -> SystemTime + Send + Sync>) -> Self {
        Self { store, fetcher, policy, clock, failures: Mutex::default(), stores: AtomicUsize::new(0) }
    }

    pub fn store(&self) -> &DiskStore {
        &self.store
    }

    /// A file for `url`, fetching it if the store has none that is fresh. Blocking: call from a worker.
    pub fn ensure(&self, url: &str, want: Want) -> Result<Cached, MediaError> {
        let address = MediaUrl::parse(url).map_err(MediaError::Blocked)?;
        let now = (self.clock)();
        let stored = self.store.get(address.as_str()).filter(|e| want.matches(e.kind));

        if let Some(entry) = stored.as_ref().filter(|e| self.age(e, now) < self.policy.fresh_for) {
            self.store.touch(entry, now);
            return Ok(cached(entry, false));
        }
        let usable_stale = stored.filter(|e| self.age(e, now) < self.policy.stale_ok_for);
        if let Some(reason) = self.remembered_failure(address.as_str(), want, now) {
            return usable_stale.map(|e| cached(&e, true)).ok_or(MediaError::Remembered(reason));
        }

        match self.download(&address, want, now) {
            Ok(entry) => Ok(cached(&entry, false)),
            Err(error) => {
                self.remember_failure(address.as_str(), want, now, &error);
                usable_stale.map(|e| cached(&e, true)).ok_or(error)
            }
        }
    }

    fn age(&self, entry: &Entry, now: SystemTime) -> Duration {
        now.duration_since(entry.fetched).unwrap_or_default()
    }

    fn download(&self, address: &MediaUrl, want: Want, now: SystemTime) -> Result<Entry, MediaError> {
        let limit = match want {
            Want::Image => self.policy.max_image_bytes,
            Want::Text => self.policy.max_text_bytes,
        };
        let fetched = self.fetcher.get(address, limit).map_err(MediaError::Fetch)?;
        // What the file is comes from its bytes, never from the server's say-so.
        let kind = match want {
            Want::Image => Kind::Image(sniff_image(&fetched.bytes).ok_or(MediaError::NotAnImage)?),
            Want::Text if looks_like_text(&fetched.bytes) => Kind::Text,
            Want::Text => return Err(MediaError::NotText),
        };
        let entry = self.store.put(address.as_str(), kind, &fetched.bytes, now).map_err(|e| MediaError::Disk(e.to_string()))?;
        if self.stores.fetch_add(1, Ordering::Relaxed) % TRIM_EVERY == TRIM_EVERY - 1 {
            // Trimming is housekeeping; a failure to do it changes nothing about this request.
            let _ = self.store.evict(self.policy.max_total_bytes);
        }
        Ok(entry)
    }

    fn remembered_failure(&self, url: &str, want: Want, now: SystemTime) -> Option<String> {
        let failures = self.failures.lock().ok()?;
        let (when, reason) = failures.get(&(url.to_string(), want))?;
        (now.duration_since(*when).unwrap_or_default() < self.policy.failure_memory).then(|| reason.clone())
    }

    fn remember_failure(&self, url: &str, want: Want, now: SystemTime, error: &MediaError) {
        if let Ok(mut failures) = self.failures.lock() {
            failures.insert((url.to_string(), want), (now, error.to_string()));
        }
    }
}

/// The file as the caller gets it, with its size read from the head of the file.
fn cached(entry: &Entry, stale: bool) -> Cached {
    let size = match entry.kind {
        Kind::Image(kind) => read_head(&entry.path).and_then(|head| dimensions(kind, &head)),
        Kind::Text => None,
    };
    Cached { path: entry.path.clone(), kind: entry.kind, stale, size }
}

fn read_head(path: &std::path::Path) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut head = Vec::new();
    std::fs::File::open(path).ok()?.take(HEAD_BYTES as u64).read_to_end(&mut head).ok()?;
    Some(head)
}

#[cfg(test)]
mod tests;
