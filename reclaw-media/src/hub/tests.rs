use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, Instant},
};

use futures_channel::oneshot::Receiver;

use super::*;
use crate::{
    cache::Policy,
    fetch::{Fetch, FetchError, Fetched},
    source::MediaUrl,
    store::DiskStore,
};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\nxxxxxxxx";

/// Answers every address with a PNG after a pause, and counts the requests.
struct SlowNet {
    calls: AtomicUsize,
    pause: Duration,
}

impl Fetch for SlowNet {
    fn get(&self, url: &MediaUrl, _max: u64) -> Result<Fetched, FetchError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        std::thread::sleep(self.pause);
        if url.as_str().contains("missing") {
            Err(FetchError::Status(404))
        } else {
            Ok(Fetched { bytes: PNG.to_vec(), content_type: None })
        }
    }
}

fn hub(pause_ms: u64, workers: usize) -> (tempfile::TempDir, Arc<SlowNet>, MediaHub) {
    let dir = tempfile::tempdir().expect("temp dir");
    let net = Arc::new(SlowNet { calls: AtomicUsize::new(0), pause: Duration::from_millis(pause_ms) });
    let cache = MediaCache::new(DiskStore::new(dir.path().join("c")), net.clone(), Policy::default());
    (dir, net, MediaHub::start(cache, workers).expect("start"))
}

/// Wait for an answer without an async runtime.
fn wait(mut rx: Receiver<Answer>) -> Answer {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match rx.try_recv() {
            Ok(Some(answer)) => return answer,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            other => panic!("no answer: {other:?}"),
        }
    }
}

#[test]
fn a_request_is_answered_from_a_worker() {
    let (_dir, net, hub) = hub(10, 2);
    let cached = wait(hub.request("https://example.com/a.png", Want::Image)).expect("fetched");
    assert!(cached.path.is_file());
    assert_eq!(net.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn two_askers_for_one_address_cause_one_fetch_and_both_are_answered() {
    let (_dir, net, hub) = hub(100, 4);
    let (a, b) = (hub.request("https://example.com/a.png", Want::Image), hub.request("https://example.com/a.png", Want::Image));
    assert_eq!(wait(a).expect("a"), wait(b).expect("b"));
    assert_eq!(net.calls.load(Ordering::SeqCst), 1);
}

#[test]
fn once_fetched_the_answer_is_there_without_waiting() {
    let (_dir, _net, hub) = hub(5, 1);
    assert_eq!(hub.peek("https://example.com/a.png", Want::Image), None);
    wait(hub.request("https://example.com/a.png", Want::Image)).expect("fetched");
    assert!(hub.peek("https://example.com/a.png", Want::Image).is_some());
    let mut again = hub.request("https://example.com/a.png", Want::Image);
    assert!(matches!(again.try_recv(), Ok(Some(Ok(_)))), "already in the channel when request returns");
}

#[test]
fn a_failure_reaches_every_waiter_and_is_not_kept_as_ready() {
    let (_dir, _net, hub) = hub(5, 2);
    let (a, b) = (hub.request("https://example.com/missing.png", Want::Image), hub.request("https://example.com/missing.png", Want::Image));
    assert_eq!(wait(a), Err(MediaError::Fetch(FetchError::Status(404))));
    assert!(wait(b).is_err());
    assert_eq!(hub.peek("https://example.com/missing.png", Want::Image), None);
}

#[test]
fn workers_fetch_different_addresses_at_the_same_time() {
    let (_dir, _net, hub) = hub(150, 4);
    let started = Instant::now();
    let all: Vec<_> = (0..4).map(|i| hub.request(&format!("https://example.com/{i}.png"), Want::Image)).collect();
    for rx in all {
        wait(rx).expect("fetched");
    }
    assert!(started.elapsed() < Duration::from_millis(500), "four in parallel took {:?}", started.elapsed());
}

#[test]
fn a_refused_address_is_answered_without_any_fetch() {
    let (_dir, net, hub) = hub(5, 1);
    assert!(matches!(wait(hub.request("http://example.com/a.png", Want::Image)), Err(MediaError::Blocked(_))));
    assert_eq!(net.calls.load(Ordering::SeqCst), 0);
}

#[test]
fn a_waiter_that_went_away_does_not_stop_the_others() {
    let (_dir, _net, hub) = hub(50, 1);
    drop(hub.request("https://example.com/a.png", Want::Image));
    let kept = hub.request("https://example.com/a.png", Want::Image);
    assert!(wait(kept).is_ok());
}
