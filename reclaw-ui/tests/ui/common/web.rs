//! A fake internet for the tests that fetch artwork and READMEs: answers from a table, records what
//! was asked for, and sits behind a real `MediaHub` on a temporary folder.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use reclaw_media::{DiskStore, Fetch, FetchError, Fetched, MediaCache, MediaHub, MediaUrl, Policy};

/// A 1x1 PNG.
const PNG_BASE64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

fn base64(text: &str) -> Vec<u8> {
    let value = |c: u8| match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    };
    let mut out = Vec::new();
    let mut bits = 0u32;
    let mut count = 0;
    for v in text.bytes().filter_map(value) {
        bits = bits << 6 | u32::from(v);
        count += 6;
        if count >= 8 {
            count -= 8;
            out.push((bits >> count) as u8);
            bits &= (1 << count) - 1;
        }
    }
    out
}

pub fn png() -> Vec<u8> {
    base64(PNG_BASE64)
}

#[derive(Default)]
pub struct FakeWeb {
    pages: Mutex<HashMap<String, Vec<u8>>>,
    asked: Mutex<Vec<String>>,
}

impl FakeWeb {
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Serve `body` at `url`. Anything not served is a 404.
    pub fn serve(&self, url: &str, body: impl Into<Vec<u8>>) {
        self.pages.lock().expect("lock").insert(url.to_string(), body.into());
    }

    /// Every address asked for so far, in order, repeats included.
    pub fn asked(&self) -> Vec<String> {
        self.asked.lock().expect("lock").clone()
    }

    pub fn asked_for(&self, url: &str) -> usize {
        self.asked().iter().filter(|u| u.as_str() == url).count()
    }
}

impl Fetch for FakeWeb {
    fn get(&self, url: &MediaUrl, max_bytes: u64) -> Result<Fetched, FetchError> {
        self.asked.lock().expect("lock").push(url.as_str().to_string());
        match self.pages.lock().expect("lock").get(url.as_str()) {
            Some(body) if body.len() as u64 > max_bytes => Err(FetchError::TooLarge { limit: max_bytes }),
            Some(body) => Ok(Fetched { bytes: body.clone(), content_type: None }),
            None => Err(FetchError::Status(404)),
        }
    }
}

/// A hub whose cache folder lives as long as this value does.
pub struct Rig {
    _dir: tempfile::TempDir,
    pub web: Arc<FakeWeb>,
    pub hub: MediaHub,
}

impl Rig {
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let web = FakeWeb::new();
        let cache = MediaCache::new(DiskStore::new(dir.path().join("media")), web.clone(), Policy::default());
        let hub = MediaHub::start(cache, 2).expect("start the hub");
        Self { _dir: dir, web, hub }
    }
}
