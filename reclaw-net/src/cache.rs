//! Answers kept on disk so the next start does not repeat the questions, and so a flaky network still shows something.
//! One file per (address, identity), written whole and moved into place, so a crash leaves the old copy or the new one.
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MAGIC: &[u8] = b"RNC1\n";

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Meta {
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub content_type: Option<String>,
    /// Unix seconds when the server last confirmed this copy.
    pub fetched_at: u64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    pub meta: Meta,
    pub body: Vec<u8>,
}

impl Entry {
    pub fn age_secs(&self, now: u64) -> u64 {
        now.saturating_sub(self.meta.fetched_at)
    }
}

pub fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[derive(Clone, Debug)]
pub struct FileCache {
    dir: PathBuf,
}

static TEMP: AtomicU64 = AtomicU64::new(0);

impl FileCache {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    fn path(&self, url: &str, credential: &str) -> PathBuf {
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        hasher.update([0]);
        hasher.update(credential.as_bytes());
        let name: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        self.dir.join(format!("{name}.rnc"))
    }

    /// The saved answer, or nothing when there is none or the file is damaged (a damaged copy is a miss, not an error).
    pub fn read(&self, url: &str, credential: &str) -> Option<Entry> {
        let bytes = fs::read(self.path(url, credential)).ok()?;
        let rest = bytes.strip_prefix(MAGIC)?;
        let newline = rest.iter().position(|b| *b == b'\n')?;
        let meta: Meta = serde_json::from_slice(&rest[..newline]).ok()?;
        (meta.url == url).then(|| Entry { meta, body: rest[newline + 1..].to_vec() })
    }

    pub fn write(&self, url: &str, credential: &str, entry: &Entry) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let path = self.path(url, credential);
        let mut bytes = MAGIC.to_vec();
        bytes.extend(serde_json::to_vec(&entry.meta).map_err(io::Error::other)?);
        bytes.push(b'\n');
        bytes.extend(&entry.body);
        write_atomically(&path, &bytes)
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

/// Write to a file beside the target and rename it over: readers see the old file or the whole new one.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let temp = path.with_extension(format!("tmp{}-{}", std::process::id(), TEMP.fetch_add(1, Ordering::Relaxed)));
    let result = fs::write(&temp, bytes).and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        // Best effort: the original error is the one worth reporting.
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests;
