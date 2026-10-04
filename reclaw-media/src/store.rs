//! The on-disk cache: one data file and one small description file per address. Files are written
//! to a temporary name and renamed into place, so a crash or a full disk never leaves half a file
//! that looks whole.
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::sniff::ImageKind;

/// What a stored file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Image(ImageKind),
    Text,
}

impl Kind {
    fn extension(self) -> &'static str {
        match self {
            Self::Image(kind) => kind.extension(),
            Self::Text => "txt",
        }
    }

    fn from_extension(ext: &str) -> Option<Self> {
        if ext == "txt" { Some(Self::Text) } else { ImageKind::from_extension(ext).map(Self::Image) }
    }
}

/// A file in the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: PathBuf,
    pub kind: Kind,
    /// When it was fetched, to the second.
    pub fetched: SystemTime,
}

/// What [`DiskStore::evict`] did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Evicted {
    pub files: usize,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DiskStore {
    root: PathBuf,
}

/// FNV-1a, 64 bit. Not for security: it names files. It is written out here rather than taken from
/// the standard library's hasher because that one makes no promise to give the same answer in the
/// next version of Rust, which would orphan the whole cache.
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3))
}

fn seconds(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl DiskStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `ab/abcdef0123456789`, without an extension: two levels so no folder grows huge.
    fn stem(&self, url: &str) -> PathBuf {
        let name = format!("{:016x}", fnv1a(url));
        self.root.join(&name[..2]).join(name)
    }

    fn meta_path(stem: &Path) -> PathBuf {
        stem.with_extension("meta")
    }

    /// The stored file for `url`, if there is one and it really is `url`'s. Two addresses can in
    /// principle share a name; the description file says whose the data is, so a clash reads as a miss.
    pub fn get(&self, url: &str) -> Option<Entry> {
        let stem = self.stem(url);
        let meta = fs::read_to_string(Self::meta_path(&stem)).ok()?;
        let mut lines = meta.lines();
        let (stored_url, fetched, ext) = (lines.next()?, lines.next()?.parse::<u64>().ok()?, lines.next()?);
        if stored_url != url {
            return None;
        }
        let kind = Kind::from_extension(ext)?;
        let path = stem.with_extension(kind.extension());
        path.is_file().then(|| Entry { path, kind, fetched: UNIX_EPOCH + Duration::from_secs(fetched) })
    }

    /// Store `bytes` as `url`'s file, replacing any earlier one.
    pub fn put(&self, url: &str, kind: Kind, bytes: &[u8], now: SystemTime) -> io::Result<Entry> {
        let stem = self.stem(url);
        let dir = stem.parent().ok_or_else(|| io::Error::other("cache path has no folder"))?;
        fs::create_dir_all(dir)?;
        // A different kind than before (a file that became an SVG) must not leave the old data behind.
        self.remove(url);
        let path = stem.with_extension(kind.extension());
        write_atomically(&path, bytes)?;
        let meta = format!("{url}\n{}\n{}\n", seconds(now), kind.extension());
        if let Err(e) = write_atomically(&Self::meta_path(&stem), meta.as_bytes()) {
            // Data without its description is unreachable; do not keep it.
            let _ = fs::remove_file(&path);
            return Err(e);
        }
        Ok(Entry { path, kind, fetched: UNIX_EPOCH + Duration::from_secs(seconds(now)) })
    }

    /// Mark a file as just used, so trimming removes the ones nobody looks at first.
    pub fn touch(&self, entry: &Entry, now: SystemTime) {
        if let Ok(file) = fs::OpenOptions::new().append(true).open(&entry.path) {
            // Failing to record a use only makes this file likelier to be trimmed early.
            let _ = file.set_modified(now);
        }
    }

    pub fn remove(&self, url: &str) {
        let stem = self.stem(url);
        for ext in ["png", "jpg", "gif", "webp", "svg", "txt", "meta"] {
            let _ = fs::remove_file(stem.with_extension(ext));
        }
    }

    /// Delete the least recently used files until the cache is at most `max_bytes`. Also deletes
    /// leftovers: temporary files from an interrupted write, and files without a description.
    pub fn evict(&self, max_bytes: u64) -> io::Result<Evicted> {
        let mut data: Vec<(PathBuf, u64, SystemTime)> = Vec::new();
        let mut done = Evicted::default();
        for shard in fs::read_dir(&self.root)?.flatten().filter(|e| e.path().is_dir()) {
            for file in fs::read_dir(shard.path())?.flatten() {
                let path = file.path();
                let Ok(meta) = file.metadata() else { continue };
                match path.extension().and_then(|e| e.to_str()) {
                    Some("meta") => {}
                    Some(ext) if Kind::from_extension(ext).is_some() => {
                        if Self::meta_path(&path).is_file() {
                            data.push((path, meta.len(), meta.modified().unwrap_or(UNIX_EPOCH)));
                        } else if fs::remove_file(&path).is_ok() {
                            done.files += 1;
                            done.bytes += meta.len();
                        }
                    }
                    // Anything else is a half-written temporary file.
                    _ => {
                        if fs::remove_file(&path).is_ok() {
                            done.files += 1;
                            done.bytes += meta.len();
                        }
                    }
                }
            }
        }
        let mut total: u64 = data.iter().map(|(_, len, _)| len).sum();
        data.sort_by_key(|(_, _, used)| *used);
        for (path, len, _) in data {
            if total <= max_bytes {
                break;
            }
            if fs::remove_file(&path).is_ok() {
                let _ = fs::remove_file(Self::meta_path(&path));
                total -= len;
                done.files += 1;
                done.bytes += len;
            }
        }
        Ok(done)
    }

    /// The total size of the data files, for showing in Settings.
    pub fn size(&self) -> u64 {
        let Ok(shards) = fs::read_dir(&self.root) else { return 0 };
        shards
            .flatten()
            .filter_map(|shard| fs::read_dir(shard.path()).ok())
            .flat_map(|files| files.flatten())
            .filter(|f| f.path().extension().and_then(|e| e.to_str()).is_some_and(|e| e != "meta"))
            .filter_map(|f| f.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

/// Write to `path.partial` and rename over `path`.
fn write_atomically(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let partial = path.with_extension("partial");
    fs::write(&partial, bytes)?;
    fs::rename(&partial, path).inspect_err(|_| {
        let _ = fs::remove_file(&partial);
    })
}

#[cfg(test)]
mod tests;
