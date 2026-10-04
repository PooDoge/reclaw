//! [`MediaHub`]: a few worker threads and a queue. The UI asks for an address and gets the answer
//! later through a channel; asking again while the first is in flight joins it instead of fetching twice.
use std::{
    collections::{HashMap, VecDeque},
    io,
    sync::{Arc, Condvar, Mutex, RwLock},
    thread,
};

use futures_channel::oneshot;

use crate::cache::{Cached, MediaCache, MediaError, Want};

type Answer = Result<Cached, MediaError>;
type Key = (String, Want);

struct Inner {
    cache: MediaCache,
    queue: Mutex<Queue>,
    wake: Condvar,
    /// Who is waiting for each address; the first to ask puts the job on the queue.
    waiting: Mutex<HashMap<Key, Vec<oneshot::Sender<Answer>>>>,
    /// Answers from this run, so a redraw finds its image at once and never waits a frame for a channel.
    ready: RwLock<HashMap<Key, Cached>>,
}

#[derive(Default)]
struct Queue {
    jobs: VecDeque<Key>,
    closed: bool,
}

/// Closes the queue when the last handle to the hub is dropped, which ends the workers.
struct Closer(Arc<Inner>);

impl Drop for Closer {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.0.queue.lock() {
            queue.closed = true;
        }
        self.0.wake.notify_all();
    }
}

#[derive(Clone)]
pub struct MediaHub {
    inner: Arc<Inner>,
    _closer: Arc<Closer>,
}

impl PartialEq for MediaHub {
    /// Two handles are equal when they are the same hub.
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.inner, &other.inner)
    }
}

impl Eq for MediaHub {}

impl MediaHub {
    /// Start `workers` threads (at least one). Fails only if the system refuses to start a thread.
    pub fn start(cache: MediaCache, workers: usize) -> io::Result<Self> {
        let inner =
            Arc::new(Inner { cache, queue: Mutex::default(), wake: Condvar::new(), waiting: Mutex::default(), ready: RwLock::default() });
        let closer = Arc::new(Closer(inner.clone()));
        for n in 0..workers.max(1) {
            let inner = inner.clone();
            thread::Builder::new().name(format!("reclaw-media-{n}")).spawn(move || work(&inner))?;
        }
        Ok(Self { inner, _closer: closer })
    }

    /// The file for `url` if it has already been fetched in this run.
    pub fn peek(&self, url: &str, want: Want) -> Option<Cached> {
        self.inner.ready.read().ok()?.get(&(url.to_string(), want)).cloned()
    }

    /// Ask for `url`. The answer arrives on the returned channel; if it is already known it is
    /// already there.
    pub fn request(&self, url: &str, want: Want) -> oneshot::Receiver<Answer> {
        let (tx, rx) = oneshot::channel();
        if let Some(known) = self.peek(url, want) {
            let _ = tx.send(Ok(known));
            return rx;
        }
        let key = (url.to_string(), want);
        let first = match self.inner.waiting.lock() {
            Ok(mut waiting) => {
                let list = waiting.entry(key.clone()).or_default();
                list.push(tx);
                list.len() == 1
            }
            // A poisoned lock means a worker panicked; the sender is dropped and the caller sees a cancelled channel.
            Err(_) => return rx,
        };
        if first && let Ok(mut queue) = self.inner.queue.lock() {
            queue.jobs.push_back(key);
            self.inner.wake.notify_one();
        }
        rx
    }
}

fn work(inner: &Inner) {
    loop {
        let job = {
            let Ok(mut queue) = inner.queue.lock() else { return };
            loop {
                if let Some(job) = queue.jobs.pop_front() {
                    break job;
                }
                if queue.closed {
                    return;
                }
                queue = match inner.wake.wait(queue) {
                    Ok(queue) => queue,
                    Err(_) => return,
                };
            }
        };
        let (url, want) = &job;
        let answer = inner.cache.ensure(url, *want);
        if let (Ok(cached), Ok(mut ready)) = (&answer, inner.ready.write()) {
            ready.insert(job.clone(), cached.clone());
        }
        let waiters = inner.waiting.lock().map(|mut w| w.remove(&job)).ok().flatten().unwrap_or_default();
        for waiter in waiters {
            // A waiter that has gone away (its widget was removed) is not an error.
            let _ = waiter.send(answer.clone());
        }
    }
}

#[cfg(test)]
mod tests;
