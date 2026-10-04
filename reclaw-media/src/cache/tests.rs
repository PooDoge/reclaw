use std::{sync::atomic::AtomicU64, time::UNIX_EPOCH};

use super::*;
use crate::{fetch::Fetched, sniff::ImageKind};

pub(crate) const PNG: &[u8] = b"\x89PNG\r\n\x1a\nxxxxxxxx";
const URL: &str = "https://example.com/a.png";

/// A network that answers from a table and counts how often it is asked.
#[derive(Default)]
pub(crate) struct FakeNet {
    pub answers: Mutex<HashMap<String, Result<Vec<u8>, FetchError>>>,
    pub calls: AtomicUsize,
}

impl FakeNet {
    pub fn answer(&self, url: &str, answer: Result<Vec<u8>, FetchError>) {
        self.answers.lock().expect("lock").insert(url.to_string(), answer);
    }

    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Fetch for FakeNet {
    fn get(&self, url: &MediaUrl, max_bytes: u64) -> Result<Fetched, FetchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.answers.lock().expect("lock").get(url.as_str()) {
            Some(Ok(bytes)) if bytes.len() as u64 > max_bytes => Err(FetchError::TooLarge { limit: max_bytes }),
            Some(Ok(bytes)) => Ok(Fetched { bytes: bytes.clone(), content_type: None }),
            Some(Err(e)) => Err(e.clone()),
            None => Err(FetchError::Status(404)),
        }
    }
}

struct Rig {
    _dir: tempfile::TempDir,
    net: Arc<FakeNet>,
    now: Arc<AtomicU64>,
    cache: MediaCache,
}

fn rig(policy: Policy) -> Rig {
    let dir = tempfile::tempdir().expect("temp dir");
    let net = Arc::new(FakeNet::default());
    let now = Arc::new(AtomicU64::new(1_000_000));
    let clock = {
        let now = now.clone();
        Arc::new(move || UNIX_EPOCH + Duration::from_secs(now.load(Ordering::SeqCst)))
    };
    let cache = MediaCache::with_clock(DiskStore::new(dir.path().join("cache")), net.clone(), policy, clock);
    Rig { _dir: dir, net, now, cache }
}

impl Rig {
    fn advance(&self, secs: u64) {
        self.now.fetch_add(secs, Ordering::SeqCst);
    }
}

#[test]
fn the_first_request_fetches_and_the_second_does_not() {
    let rig = rig(Policy::default());
    rig.net.answer(URL, Ok(PNG.to_vec()));
    let first = rig.cache.ensure(URL, Want::Image).expect("fetched");
    assert_eq!(first.kind, Kind::Image(ImageKind::Png));
    assert!(!first.stale);
    assert_eq!(std::fs::read(&first.path).expect("file"), PNG);
    let second = rig.cache.ensure(URL, Want::Image).expect("cached");
    assert_eq!(second.path, first.path);
    assert_eq!(rig.net.calls(), 1);
}

#[test]
fn the_file_kind_comes_from_the_bytes_not_the_address() {
    let rig = rig(Policy::default());
    rig.net.answer("https://example.com/not-a-png.png", Ok(b"<html>hello</html>".to_vec()));
    assert_eq!(rig.cache.ensure("https://example.com/not-a-png.png", Want::Image), Err(MediaError::NotAnImage));
    rig.net.answer("https://example.com/real", Ok(b"<svg xmlns='http://www.w3.org/2000/svg'/>".to_vec()));
    assert_eq!(rig.cache.ensure("https://example.com/real", Want::Image).expect("svg").kind, Kind::Image(ImageKind::Svg));
}

#[test]
fn text_is_kept_as_text_and_binary_is_refused() {
    let rig = rig(Policy::default());
    rig.net.answer("https://example.com/README.md", Ok(b"# Hello\n".to_vec()));
    rig.net.answer("https://example.com/blob", Ok(PNG.to_vec()));
    assert_eq!(rig.cache.ensure("https://example.com/README.md", Want::Text).expect("text").kind, Kind::Text);
    assert_eq!(rig.cache.ensure("https://example.com/blob", Want::Text), Err(MediaError::NotText));
}

#[test]
fn a_file_cached_as_one_thing_is_not_served_as_another() {
    let rig = rig(Policy::default());
    rig.net.answer(URL, Ok(PNG.to_vec()));
    rig.cache.ensure(URL, Want::Image).expect("image");
    assert_eq!(rig.cache.ensure(URL, Want::Text), Err(MediaError::NotText), "asked for text, the server's bytes are an image");
}

#[test]
fn refused_addresses_never_reach_the_network() {
    let rig = rig(Policy::default());
    for bad in ["http://example.com/a.png", "https://192.168.0.1/a.png", "file:///etc/passwd"] {
        assert!(matches!(rig.cache.ensure(bad, Want::Image), Err(MediaError::Blocked(_))), "{bad}");
    }
    assert_eq!(rig.net.calls(), 0);
}

#[test]
fn an_oversize_body_is_refused_and_not_stored() {
    let rig = rig(Policy { max_image_bytes: 10, ..Policy::default() });
    rig.net.answer(URL, Ok(PNG.to_vec()));
    assert_eq!(rig.cache.ensure(URL, Want::Image), Err(MediaError::Fetch(FetchError::TooLarge { limit: 10 })));
    assert_eq!(rig.cache.store().size(), 0);
}

#[test]
fn a_stale_file_is_fetched_again() {
    let rig = rig(Policy::default());
    rig.net.answer(URL, Ok(PNG.to_vec()));
    rig.cache.ensure(URL, Want::Image).expect("first");
    rig.advance(2 * 24 * 60 * 60);
    rig.cache.ensure(URL, Want::Image).expect("refetched");
    assert_eq!(rig.net.calls(), 2);
}

#[test]
fn when_the_refetch_fails_the_old_file_is_served_and_marked_stale() {
    let rig = rig(Policy::default());
    rig.net.answer(URL, Ok(PNG.to_vec()));
    let first = rig.cache.ensure(URL, Want::Image).expect("first");
    rig.advance(2 * 24 * 60 * 60);
    rig.net.answer(URL, Err(FetchError::Timeout));
    let again = rig.cache.ensure(URL, Want::Image).expect("falls back");
    assert_eq!((again.path, again.stale), (first.path, true));
}

#[test]
fn a_file_too_old_to_fall_back_on_is_an_error() {
    let rig = rig(Policy::default());
    rig.net.answer(URL, Ok(PNG.to_vec()));
    rig.cache.ensure(URL, Want::Image).expect("first");
    rig.advance(60 * 24 * 60 * 60);
    rig.net.answer(URL, Err(FetchError::Timeout));
    assert_eq!(rig.cache.ensure(URL, Want::Image), Err(MediaError::Fetch(FetchError::Timeout)));
}

#[test]
fn a_failure_is_remembered_for_a_while_and_then_tried_again() {
    let rig = rig(Policy::default());
    assert_eq!(rig.cache.ensure(URL, Want::Image), Err(MediaError::Fetch(FetchError::Status(404))));
    assert_eq!(rig.net.calls(), 1);
    let again = rig.cache.ensure(URL, Want::Image).expect_err("remembered");
    assert!(matches!(again, MediaError::Remembered(_)), "{again:?}");
    assert_eq!(rig.net.calls(), 1, "no second request");

    rig.advance(11 * 60);
    rig.net.answer(URL, Ok(PNG.to_vec()));
    assert!(rig.cache.ensure(URL, Want::Image).is_ok(), "the memory expired and the image arrived");
    assert_eq!(rig.net.calls(), 2);
}

#[test]
fn the_cache_is_trimmed_now_and_then() {
    let rig = rig(Policy { max_total_bytes: 100, ..Policy::default() });
    for i in 0..TRIM_EVERY {
        let url = format!("https://example.com/{i}.png");
        let mut png = PNG.to_vec();
        png.extend_from_slice(&[0u8; 100]);
        rig.net.answer(&url, Ok(png));
        rig.cache.ensure(&url, Want::Image).expect("fetched");
    }
    assert!(rig.cache.store().size() <= 200, "trimmed to about the limit, size {}", rig.cache.store().size());
}
