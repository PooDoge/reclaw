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
    /// Requests before this moment fail at once, with this error, instead of making things worse.
    blocked: Option<(Instant, NetError)>,
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
            .or_insert_with(|| Host { slots: Arc::new(Semaphore::new(self.per_host)), blocked: None });
        f(entry)
    }

    /// What a request to this host would be told right now, if it is blocked.
    pub fn blocked(&self, host: &str) -> Option<NetError> {
        self.with_host(host, |h| {
            let (until, error) = h.blocked.as_ref()?;
            let left = until.checked_duration_since(Instant::now())?;
            Some(match error {
                // The wait is what is left of it, not what it was when it began.
                NetError::RateLimited { host, .. } => NetError::RateLimited { host: host.clone(), retry_in: left },
                other => other.clone(),
            })
        })
    }

    /// Remember that `host` wants `wait` before the next request.
    pub fn block(&self, host: &str, wait: Duration) {
        self.block_with(host, wait, NetError::RateLimited { host: host.to_string(), retry_in: wait });
    }

    /// Remember that the network refuses this host: every request in the next `for_` fails at once with `error`. A refusal by a
    /// proxy or firewall is policy; asking again a hundred times (once per picture) changes nothing except the other side's logs.
    pub fn deny(&self, host: &str, error: NetError, for_: Duration) {
        self.block_with(host, for_, error);
    }

    fn block_with(&self, host: &str, wait: Duration, error: NetError) {
        let until = Instant::now() + wait;
        self.with_host(host, |h| {
            if h.blocked.as_ref().is_none_or(|(t, _)| *t < until) {
                h.blocked = Some((until, error));
            }
        });
    }

    /// Take a place in line for `host`, or say how long it is blocked.
    pub async fn enter(&self, host: &str) -> Result<OwnedSemaphorePermit, NetError> {
        if let Some(error) = self.blocked(host) {
            return Err(error);
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
        assert!(gate.blocked("API.GITHUB.COM").is_some(), "host names compare ignoring case");
    }

    #[tokio::test]
    async fn the_longer_of_two_blocks_wins() {
        let gate = HostGate::new(1);
        gate.block("h.example", Duration::from_secs(100));
        gate.block("h.example", Duration::from_secs(1));
        assert!(matches!(gate.blocked("h.example"), Some(NetError::RateLimited { retry_in, .. }) if retry_in > Duration::from_secs(50)));
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

    #[tokio::test]
    async fn a_refused_host_answers_with_the_same_refusal_until_the_time_is_up() {
        let gate = HostGate::new(2);
        let refusal = NetError::ProxyDenied { host: "cdn.example".into(), status: Some(403) };
        gate.deny("cdn.example", refusal.clone(), Duration::from_millis(80));
        assert_eq!(gate.enter("cdn.example").await.err(), Some(refusal.clone()));
        assert!(gate.enter("other.example").await.is_ok(), "only that host");
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(gate.enter("cdn.example").await.is_ok(), "and not for ever");
    }
}
