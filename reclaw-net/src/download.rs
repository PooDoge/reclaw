//! Downloading a file of any size. The body goes straight to disk (never held in memory), is hashed as it arrives, and a
//! transfer that is cut off, stalls or is cancelled continues where it stopped: the partial file is kept beside the target
//! with a small note of what it is a part of, and the next attempt asks the server for the rest (`Range`, guarded by
//! `If-Range` so a file that changed meanwhile restarts instead of being stitched together wrongly).
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use reqwest::header::{ACCEPT, ACCEPT_ENCODING, CONTENT_RANGE, ETAG, HeaderValue, IF_RANGE, LAST_MODIFIED, RANGE};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::{
    cache::write_atomically,
    error::{NetError, from_io},
    net::{Attempt, Net},
    retry,
};

/// How often the progress callback is called at most.
const REPORT_EVERY: Duration = Duration::from_millis(100);
/// How often a silent server is checked against the stall limit and the cancel flag.
const TICK: Duration = Duration::from_millis(250);
/// A hard stop for a download that keeps "making progress" a few bytes at a time.
const MAX_ATTEMPTS: u32 = 60;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct DownloadRequest {
    pub url: String,
    /// Where the finished file goes. Its folder is created. `<dest>.part` and `<dest>.part.json` exist while it is incomplete.
    pub dest: PathBuf,
    /// Lower-case hex SHA-256 (a `sha256:` prefix, as GitHub writes it, is accepted). A different file is an error and is deleted.
    pub expected_sha256: Option<String>,
    pub expected_size: Option<u64>,
    pub max_bytes: u64,
}

impl DownloadRequest {
    pub fn new(url: impl Into<String>, dest: impl Into<PathBuf>) -> Self {
        Self { url: url.into(), dest: dest.into(), expected_sha256: None, expected_size: None, max_bytes: 16 * 1024 * 1024 * 1024 }
    }

    pub fn sha256(mut self, digest: &str) -> Self {
        self.expected_sha256 = Some(digest.trim().trim_start_matches("sha256:").to_ascii_lowercase());
        self
    }

    pub fn size(mut self, bytes: u64) -> Self {
        self.expected_size = Some(bytes);
        self
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Progress {
    /// Bytes on disk, including any from an earlier attempt.
    pub downloaded: u64,
    pub total: Option<u64>,
    /// Where this transfer picked up, 0 when it started from the beginning.
    pub resumed_from: u64,
    /// Smoothed over the last few seconds.
    pub bytes_per_sec: f64,
}

impl Progress {
    pub fn fraction(&self) -> Option<f64> {
        self.total.filter(|t| *t > 0).map(|t| (self.downloaded as f64 / t as f64).clamp(0.0, 1.0))
    }

    pub fn eta(&self) -> Option<Duration> {
        let left = self.total?.saturating_sub(self.downloaded);
        (self.bytes_per_sec > 1.0).then(|| Duration::from_secs_f64(left as f64 / self.bytes_per_sec))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Downloaded {
    pub path: PathBuf,
    pub bytes: u64,
    /// Lower-case hex.
    pub sha256: String,
    pub resumed_from: u64,
    /// The file was already there and matched, so nothing was fetched.
    pub already_present: bool,
}

/// Ask a running download to stop. Clones share the flag. The partial file is kept for a later resume.
#[derive(Clone, Debug, Default)]
pub struct Cancel(Arc<AtomicBool>);

impl Cancel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

/// What is written beside a partial file so the next attempt knows what it is a part of.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
struct PartNote {
    url: String,
    etag: Option<String>,
    last_modified: Option<String>,
    total: Option<u64>,
}

/// Why an attempt ended without a finished file.
struct Failed {
    error: NetError,
    retry: bool,
    /// Bytes were added to the partial file, so the failure counter starts again.
    progressed: bool,
    wait: Option<Duration>,
}

impl Failed {
    fn stop(error: NetError) -> Self {
        Self { error, retry: false, progressed: false, wait: None }
    }
}

fn beside(dest: &Path, suffix: &str) -> PathBuf {
    let mut name = dest.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// `bytes START-END/TOTAL`, as in a `Content-Range` header.
fn parse_content_range(value: &str) -> Option<(u64, u64)> {
    let rest = value.trim().strip_prefix("bytes")?.trim_start();
    let (range, total) = rest.split_once('/')?;
    let start = range.split_once('-')?.0.trim().parse().ok()?;
    Some((start, total.trim().parse().ok()?))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The hash of a file's bytes, computed off the runtime's threads.
async fn hash_file(path: PathBuf) -> Result<(Sha256, u64), NetError> {
    tokio::task::spawn_blocking(move || {
        use std::io::Read;
        let mut file = std::fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let (mut buffer, mut total) = (vec![0u8; 1 << 20], 0u64);
        loop {
            let n = file.read(&mut buffer)?;
            if n == 0 {
                return Ok((hasher, total));
            }
            hasher.update(&buffer[..n]);
            total += n as u64;
        }
    })
    .await
    .map_err(|e| NetError::Other(format!("hashing was interrupted: {e}")))?
    .map_err(|e: std::io::Error| from_io(&e))
}

impl Net {
    /// Download a file. Call from a worker thread; `on_progress` runs on that thread, a few times a second. Progress is kept
    /// across failures: a dropped connection continues from the same byte, and `cancel` stops it with the partial file kept.
    pub fn download(
        &self,
        request: &DownloadRequest,
        cancel: &Cancel,
        on_progress: &mut dyn FnMut(&Progress),
    ) -> Result<Downloaded, NetError> {
        self.block(self.download_async(request, cancel, on_progress))?
    }

    async fn download_async(
        &self,
        request: &DownloadRequest,
        cancel: &Cancel,
        on_progress: &mut dyn FnMut(&Progress),
    ) -> Result<Downloaded, NetError> {
        let url = self.parse(&request.url)?;
        let (label, started) = (crate::address::describe(&url), Instant::now());
        tracing::info!(url = %label, dest = %request.dest.display(), expected_bytes = request.expected_size, "download starting");
        let result = self.download_loop(request, &url, cancel, on_progress).await;
        let ms = started.elapsed().as_millis() as u64;
        match &result {
            Ok(done) if done.already_present => tracing::info!(url = %label, bytes = done.bytes, "already downloaded and verified"),
            Ok(done) => tracing::info!(url = %label, bytes = done.bytes, resumed_from = done.resumed_from, ms, "download finished"),
            Err(NetError::Cancelled) => tracing::info!(url = %label, ms, "download cancelled; the partial file is kept"),
            Err(error) => tracing::warn!(url = %label, %error, hint = error.hint(), ms, "download failed"),
        }
        result
    }

    async fn download_loop(
        &self,
        request: &DownloadRequest,
        url: &url::Url,
        cancel: &Cancel,
        on_progress: &mut dyn FnMut(&Progress),
    ) -> Result<Downloaded, NetError> {
        if let Some(dir) = request.dest.parent().filter(|d| !d.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(dir).await.map_err(|e| from_io(&e))?;
        }
        if let Some(present) = self.already_there(request).await? {
            return Ok(present);
        }
        let allowed = self.inner.cfg.attempts.max(1);
        let (mut failures, mut total_attempts) = (0u32, 0u32);
        loop {
            if cancel.is_cancelled() {
                return Err(NetError::Cancelled);
            }
            total_attempts += 1;
            match self.download_once(request, url, cancel, on_progress).await {
                Ok(done) => return Ok(done),
                Err(failed) => {
                    if !failed.retry || cancel.is_cancelled() {
                        return Err(failed.error);
                    }
                    failures = if failed.progressed { 0 } else { failures + 1 };
                    if failures >= allowed || total_attempts >= MAX_ATTEMPTS {
                        return Err(failed.error);
                    }
                    let wait = failed.wait.unwrap_or_else(|| retry::delay(failures.saturating_sub(1), retry::jitter()));
                    tracing::warn!(
                        url = %crate::address::describe(url),
                        attempt = total_attempts,
                        progressed = failed.progressed,
                        wait_ms = wait.as_millis() as u64,
                        error = %failed.error,
                        "download interrupted; continuing"
                    );
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    /// A finished file that is already in place and matches what was promised needs no download.
    async fn already_there(&self, request: &DownloadRequest) -> Result<Option<Downloaded>, NetError> {
        let Ok(meta) = tokio::fs::metadata(&request.dest).await else { return Ok(None) };
        let Some(expected) = &request.expected_sha256 else { return Ok(None) };
        if request.expected_size.is_some_and(|s| s != meta.len()) {
            return Ok(None);
        }
        let (hasher, bytes) = hash_file(request.dest.clone()).await?;
        let digest = hex(&hasher.finalize());
        Ok((&digest == expected).then(|| Downloaded {
            path: request.dest.clone(),
            bytes,
            sha256: digest,
            resumed_from: 0,
            already_present: true,
        }))
    }

    async fn download_once(
        &self,
        request: &DownloadRequest,
        url: &url::Url,
        cancel: &Cancel,
        on_progress: &mut dyn FnMut(&Progress),
    ) -> Result<Downloaded, Failed> {
        let host = url.host_str().unwrap_or_default().to_string();
        let part = beside(&request.dest, ".part");
        let note_path = beside(&request.dest, ".part.json");

        // What an earlier attempt left, if it is a part of this very file.
        let note: Option<PartNote> = tokio::fs::read(&note_path).await.ok().and_then(|b| serde_json::from_slice(&b).ok());
        let on_disk = tokio::fs::metadata(&part).await.map(|m| m.len()).unwrap_or(0);
        let resume_from = match &note {
            Some(n) if n.url == url.as_str() && on_disk > 0 => on_disk,
            _ => 0,
        };
        if resume_from > 0 {
            tracing::info!(url = %crate::address::describe(url), resume_from, "continuing a partial download");
        } else if on_disk > 0 {
            tracing::info!(url = %crate::address::describe(url), on_disk, "a partial file is not of this download; starting over");
        }

        let mut builder =
            self.inner.files.get(url.clone()).header(ACCEPT_ENCODING, "identity").header(ACCEPT, "application/octet-stream, */*;q=0.8");
        if let Some((name, value)) = self.auth_header(url) {
            builder = builder.header(name, value);
        }
        if resume_from > 0 {
            builder = builder.header(RANGE, format!("bytes={resume_from}-"));
            let validator = note.as_ref().and_then(|n| n.etag.clone().or_else(|| n.last_modified.clone()));
            if let Some(validator) = validator.and_then(|v| HeaderValue::from_str(&v).ok()) {
                builder = builder.header(IF_RANGE, validator);
            }
        }

        let permit = self.inner.gate.enter(&host).await.map_err(Failed::stop)?;
        let response = builder.send().await.map_err(|e| attempt_to_failed(self.transport(&host, &e)))?;
        let status = response.status().as_u16();

        if status == 416 {
            // Nothing past what we have. If we have it all, finish; otherwise the partial file is not usable.
            if note.as_ref().and_then(|n| n.total) == Some(resume_from) {
                drop(permit);
                return finish(request, &part, &note_path, resume_from, resume_from, None).await;
            }
            discard(&part, &note_path).await;
            return Err(Failed { error: NetError::Status { host, status }, retry: true, progressed: false, wait: Some(Duration::ZERO) });
        }
        if !response.status().is_success() {
            return Err(attempt_to_failed(self.rejected(&host, response).await));
        }

        let header = |name| response.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
        let (etag, last_modified) = (header(ETAG), header(LAST_MODIFIED));
        let (start, total) = if status == 206 {
            match header(CONTENT_RANGE).as_deref().and_then(parse_content_range) {
                Some((start, total)) if start == resume_from => (start, Some(total)),
                _ => {
                    discard(&part, &note_path).await;
                    return Err(Failed {
                        error: NetError::Other("the server answered a range request with the wrong range".into()),
                        retry: true,
                        progressed: false,
                        wait: Some(Duration::ZERO),
                    });
                }
            }
        } else {
            // A 200 means the whole file, whatever was asked: the server ignored the range, or the file changed.
            (0, response.content_length())
        };
        if total.is_some_and(|t| t > request.max_bytes) {
            return Err(Failed::stop(NetError::TooLarge { limit: request.max_bytes }));
        }
        if let (Some(expected), Some(total)) = (request.expected_size, total)
            && expected != total
        {
            return Err(Failed::stop(NetError::Integrity(format!("expected {expected} bytes but the server has {total}"))));
        }

        let note = PartNote { url: url.to_string(), etag, last_modified, total };
        let note_bytes = serde_json::to_vec(&note).map_err(|e| Failed::stop(NetError::Other(e.to_string())))?;
        let write_note = {
            let path = note_path.clone();
            tokio::task::spawn_blocking(move || write_atomically(&path, &note_bytes)).await
        };
        write_note.map_err(|e| Failed::stop(NetError::Other(e.to_string())))?.map_err(|e| Failed::stop(from_io(&e)))?;

        let (mut hasher, mut downloaded) = if start > 0 {
            let (hasher, have) = hash_file(part.clone()).await.map_err(Failed::stop)?;
            if have < start {
                discard(&part, &note_path).await;
                return Err(Failed {
                    error: NetError::Other("the partial file is shorter than it was".into()),
                    retry: true,
                    progressed: false,
                    wait: Some(Duration::ZERO),
                });
            }
            (hasher, start)
        } else {
            (Sha256::new(), 0)
        };
        let file =
            if start > 0 { tokio::fs::OpenOptions::new().append(true).open(&part).await } else { tokio::fs::File::create(&part).await }
                .map_err(|e| Failed::stop(from_io(&e)))?;
        let mut writer = BufWriter::with_capacity(1 << 20, file);

        let (mut response, begun, mut last_report, mut idle) = (response, Instant::now(), Instant::now() - REPORT_EVERY, Duration::ZERO);
        let (mut speed, mut last_bytes, mut last_tick) = (0.0f64, downloaded, Instant::now());
        let stall = self.inner.cfg.stall_timeout;
        loop {
            if cancel.is_cancelled() {
                let _ = writer.flush().await;
                return Err(Failed { error: NetError::Cancelled, retry: false, progressed: downloaded > start, wait: None });
            }
            match tokio::time::timeout(TICK, response.chunk()).await {
                Err(_) => {
                    idle += TICK;
                    if idle >= stall {
                        let _ = writer.flush().await;
                        return Err(Failed { error: NetError::Stalled(stall), retry: true, progressed: downloaded > start, wait: None });
                    }
                }
                Ok(Ok(Some(chunk))) => {
                    idle = Duration::ZERO;
                    downloaded += chunk.len() as u64;
                    if downloaded > request.max_bytes {
                        discard(&part, &note_path).await;
                        return Err(Failed::stop(NetError::TooLarge { limit: request.max_bytes }));
                    }
                    hasher.update(&chunk);
                    writer.write_all(&chunk).await.map_err(|e| Failed::stop(from_io(&e)))?;
                    if last_report.elapsed() >= REPORT_EVERY {
                        let seconds = last_tick.elapsed().as_secs_f64().max(0.001);
                        let instant = (downloaded - last_bytes) as f64 / seconds;
                        speed = if speed == 0.0 { instant } else { 0.7 * speed + 0.3 * instant };
                        (last_bytes, last_tick, last_report) = (downloaded, Instant::now(), Instant::now());
                        on_progress(&Progress { downloaded, total, resumed_from: start, bytes_per_sec: speed });
                    }
                }
                Ok(Ok(None)) => break,
                Ok(Err(e)) => {
                    let _ = writer.flush().await;
                    let mut failed = attempt_to_failed(self.transport(&host, &e));
                    failed.progressed = downloaded > start;
                    // A connection that drops mid-body is worth another go even if its message looks unfamiliar.
                    failed.retry = failed.retry || failed.progressed;
                    return Err(failed);
                }
            }
        }
        writer.flush().await.map_err(|e| Failed::stop(from_io(&e)))?;
        writer.get_ref().sync_all().await.map_err(|e| Failed::stop(from_io(&e)))?;
        drop(writer);
        drop(permit);
        let seconds = begun.elapsed().as_secs_f64().max(0.001);
        on_progress(&Progress {
            downloaded,
            total,
            resumed_from: start,
            bytes_per_sec: if speed > 0.0 { speed } else { (downloaded - start) as f64 / seconds },
        });

        if let Some(total) = total {
            if downloaded < total {
                return Err(Failed {
                    error: NetError::Other(format!("the connection closed after {downloaded} of {total} bytes")),
                    retry: true,
                    progressed: downloaded > start,
                    wait: None,
                });
            }
            if downloaded > total {
                discard(&part, &note_path).await;
                return Err(Failed::stop(NetError::Integrity(format!("received {downloaded} bytes but {total} were announced"))));
            }
        }
        finish(request, &part, &note_path, downloaded, start, Some(hex(&hasher.finalize()))).await
    }
}

/// Check what arrived against what was promised and move it into place.
async fn finish(
    request: &DownloadRequest,
    part: &Path,
    note_path: &Path,
    bytes: u64,
    resumed_from: u64,
    digest: Option<String>,
) -> Result<Downloaded, Failed> {
    let digest = match digest {
        Some(d) => d,
        None => hash_file(part.to_path_buf()).await.map(|(h, _)| hex(&h.finalize())).map_err(Failed::stop)?,
    };
    if let Some(expected) = request.expected_size.filter(|s| *s != bytes) {
        discard(part, note_path).await;
        return Err(Failed::stop(NetError::Integrity(format!("expected {expected} bytes, got {bytes}"))));
    }
    if let Some(expected) = request.expected_sha256.as_ref().filter(|e| **e != digest) {
        discard(part, note_path).await;
        return Err(Failed::stop(NetError::Integrity(format!("the SHA-256 is {digest}, not the expected {expected}"))));
    }
    tokio::fs::rename(part, &request.dest).await.map_err(|e| Failed::stop(from_io(&e)))?;
    // The note is only a hint for resuming; a leftover one is harmless, so a failure to remove it is not an error.
    let _ = tokio::fs::remove_file(note_path).await;
    Ok(Downloaded { path: request.dest.clone(), bytes, sha256: digest, resumed_from, already_present: false })
}

async fn discard(part: &Path, note: &Path) {
    // Best effort: these are scratch files, and the caller already has the error that matters.
    let _ = tokio::fs::remove_file(part).await;
    let _ = tokio::fs::remove_file(note).await;
}

fn attempt_to_failed(attempt: Attempt) -> Failed {
    match attempt {
        Attempt::Retry { error, wait } => Failed { error, retry: true, progressed: false, wait },
        Attempt::Stop(error) => Failed::stop(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_range_is_read_for_start_and_total() {
        assert_eq!(parse_content_range("bytes 100-199/1000"), Some((100, 1000)));
        assert_eq!(parse_content_range("bytes 0-0/1"), Some((0, 1)));
        for bad in ["", "bytes */1000", "items 0-1/2", "bytes 0-1/x", "bytes 1-2"] {
            assert_eq!(parse_content_range(bad), None, "{bad:?}");
        }
    }

    #[test]
    fn a_github_style_digest_is_accepted_and_normalised() {
        let request = DownloadRequest::new("https://e.test/f", "/tmp/f").sha256("sha256:ABCDEF");
        assert_eq!(request.expected_sha256.as_deref(), Some("abcdef"));
    }

    #[test]
    fn progress_knows_its_fraction_and_time_left() {
        let p = Progress { downloaded: 250, total: Some(1000), resumed_from: 0, bytes_per_sec: 50.0 };
        assert_eq!(p.fraction(), Some(0.25));
        assert_eq!(p.eta(), Some(Duration::from_secs(15)));
        let unknown = Progress { downloaded: 5, total: None, resumed_from: 0, bytes_per_sec: 0.0 };
        assert_eq!((unknown.fraction(), unknown.eta()), (None, None));
    }

    #[test]
    fn cancelling_one_handle_cancels_its_clones() {
        let a = Cancel::new();
        let b = a.clone();
        assert!(!b.is_cancelled());
        a.cancel();
        assert!(b.is_cancelled());
    }
}
