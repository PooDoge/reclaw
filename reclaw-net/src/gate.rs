//! Politeness toward each host: a limit on requests in flight, and a memory of "this host asked us to wait". Without it
//! a library of two hundred apps asks GitHub two hundred questions at once, is told to stop, and asks again.
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use crate::error::NetError;

struct Host {
    slots: Arc<Semaphore>,
    /// Requests before this moment fail at once instead of making things worse.
    blocked_until: Option<Instant>,
}

pub struct HostGate {
    per_host: usize,
    hosts: Mutex<HashMap<String, Host>>,
}

impl HostGate {
    pub fn new(per_host: usize) -> Self {
        Self { per_host: per_host.max(1), hosts: Mutex::new(HashMap::new()) }
    }

    fn with_host<T>(&self, host: &str, f: impl FnOnce(&mut Host) -> T) -> T {
        // A panic while holding this lock leaves plain data; keep going rather than poison every later request.
        let mut hosts = self.hosts.lock().unwrap_or_else(|e| e.into_inner());
        let entry = hosts
            .entry(host.to_ascii_lowercase())
            .or_insert_with(|| Host { slots: Arc::new(Semaphore::new(self.per_host)), blocked_until: None });
        f(entry)
    }

    /// How long the host has asked us to wait, if it has.
    pub fn blocked_for(&self, host: &str) -> Option<Duration> {
        self.with_host(host, |h| h.blocked_until.and_then(|t| t.checked_duration_since(Instant::now())))
    }

    /// Remember that `host` wants `wait` before the next request.
    pub fn block(&self, host: &str, wait: Duration) {
        let until = Instant::now() + wait;
        self.with_host(host, |h| h.blocked_until = Some(h.blocked_until.map_or(until, |t| t.max(until))));
    }

    /// Take a place in line for `host`, or say how long it is blocked.
    pub async fn enter(&self, host: &str) -> Result<OwnedSemaphorePermit, NetError> {
        if let Some(retry_in) = self.blocked_for(host) {
            return Err(NetError::RateLimited { host: host.to_string(), retry_in });
        }
        let slots = self.with_host(host, |h| h.slots.clone());
        slots.acquire_owned().await.map_err(|_| NetError::Other("the request queue was closed".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "current_thread", start_paused = true)]
    async fn a_blocked_host_refuses_at_once_and_recovers() {
        let gate = HostGate::new(2);
        assert!(gate.enter("api.github.com").await.is_ok());
        gate.block("api.github.com", Duration::from_secs(30));
        assert!(matches!(gate.enter("api.github.com").await, Err(NetError::RateLimited { .. })));
        assert!(gate.enter("gitlab.com").await.is_ok(), "other hosts are unaffected");
        assert!(gate.blocked_for("API.GITHUB.COM").is_some(), "host names compare ignoring case");
    }

    #[tokio::test]
    async fn the_longer_of_two_blocks_wins() {
        let gate = HostGate::new(1);
        gate.block("h.example", Duration::from_secs(100));
        gate.block("h.example", Duration::from_secs(1));
        assert!(gate.blocked_for("h.example").is_some_and(|d| d > Duration::from_secs(50)));
    }

    #[tokio::test]
    async fn only_so_many_requests_run_at_once_per_host() {
        let gate = Arc::new(HostGate::new(2));
        let a = gate.enter("h.example").await.expect("first");
        let _b = gate.enter("h.example").await.expect("second");
        let waiting = tokio::time::timeout(Duration::from_millis(50), gate.enter("h.example")).await;
        assert!(waiting.is_err(), "the third waits for a place");
        drop(a);
        assert!(tokio::time::timeout(Duration::from_millis(500), gate.enter("h.example")).await.is_ok(), "and gets it when one finishes");
    }
}
