use std::{
    sync::mpsc::{self, RecvTimeoutError, Sender},
    thread::JoinHandle,
    time::Duration,
};

use crate::{error::PrefsError, file::PrefsFile, prefs::Preferences};

enum Message {
    Save(Box<Preferences>),
    /// Write what is waiting now, then say so.
    Flush(Sender<()>),
    Stop,
}

/// Saves preferences on a background thread. The UI calls [`submit`](Self::submit) on every change;
/// changes that arrive within `delay` of each other are written once, with the latest value, so
/// dragging a slider does not write the file a hundred times. Dropping the writer writes anything
/// still waiting and stops the thread.
pub struct PrefsWriter {
    tx: Sender<Message>,
    thread: Option<JoinHandle<()>>,
}

impl PrefsWriter {
    /// `on_error` is called on the writer thread when a save fails; it must not block.
    pub fn spawn(file: PrefsFile, delay: Duration, on_error: impl Fn(PrefsError) + Send + 'static) -> Self {
        let (tx, rx) = mpsc::channel::<Message>();
        let thread = std::thread::Builder::new().name("reclaw-prefs-writer".into()).spawn(move || {
            let mut pending: Option<Box<Preferences>> = None;
            loop {
                let message = if pending.is_some() {
                    match rx.recv_timeout(delay) {
                        Ok(message) => Some(message),
                        Err(RecvTimeoutError::Timeout) => None,
                        Err(RecvTimeoutError::Disconnected) => Some(Message::Stop),
                    }
                } else {
                    Some(rx.recv().unwrap_or(Message::Stop))
                };
                match message {
                    Some(Message::Save(prefs)) => pending = Some(prefs),
                    Some(Message::Flush(done)) => {
                        if let Some(prefs) = pending.take() {
                            report(file.save(&prefs), &on_error);
                        }
                        // The caller may have given up waiting; that is fine.
                        let _ = done.send(());
                    }
                    Some(Message::Stop) => {
                        if let Some(prefs) = pending.take() {
                            report(file.save(&prefs), &on_error);
                        }
                        return;
                    }
                    // Quiet for `delay`: write the latest.
                    None => {
                        if let Some(prefs) = pending.take() {
                            report(file.save(&prefs), &on_error);
                        }
                    }
                }
            }
        });
        // If the thread cannot be created there is nothing to write with: saving silently does not
        // happen. That is reported once through `on_error`-less paths by the caller checking `is_running`.
        Self { tx, thread: thread.ok() }
    }

    /// Whether the background thread exists. False only if the OS refused to start it.
    pub fn is_running(&self) -> bool {
        self.thread.is_some()
    }

    /// Write anything waiting and return once it is on disk (or after two seconds, if the thread is
    /// stuck). For shutdown: the thread holds a change back to merge a burst of them.
    pub fn flush(&self) {
        let (done, wait) = mpsc::channel();
        if self.tx.send(Message::Flush(done)).is_ok() {
            let _ = wait.recv_timeout(Duration::from_secs(2));
        }
    }

    pub fn submit(&self, prefs: Preferences) {
        // A closed channel means the thread ended; there is nobody to tell and nothing to write to.
        let _ = self.tx.send(Message::Save(Box::new(prefs)));
    }
}

fn report(result: Result<(), PrefsError>, on_error: &impl Fn(PrefsError)) {
    if let Err(e) = result {
        on_error(e);
    }
}

impl Drop for PrefsWriter {
    fn drop(&mut self) {
        let _ = self.tx.send(Message::Stop);
        if let Some(thread) = self.thread.take() {
            // A panic on the writer thread has already been reported by the runtime.
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    fn with_favorite(id: u32) -> Preferences {
        let mut p = Preferences::default();
        p.favorites.insert(id);
        p
    }

    #[test]
    fn rapid_changes_are_written_once_with_the_latest_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = PrefsFile::at(dir.path().join("settings.toml"));
        let writer = PrefsWriter::spawn(file.clone(), Duration::from_millis(80), |e| panic!("{e}"));
        for id in 1..=20 {
            writer.submit(with_favorite(id));
        }
        drop(writer);
        let favorites: Vec<u32> = file.load().prefs.favorites.into_iter().collect();
        assert_eq!(favorites, vec![20]);
    }

    #[test]
    fn dropping_flushes_what_is_waiting() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = PrefsFile::at(dir.path().join("settings.toml"));
        let writer = PrefsWriter::spawn(file.clone(), Duration::from_secs(60), |e| panic!("{e}"));
        writer.submit(with_favorite(7));
        drop(writer);
        assert!(file.load().prefs.favorites.contains(&7), "the long delay did not lose the change");
    }

    #[test]
    fn flush_writes_now_without_waiting_for_the_delay() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = PrefsFile::at(dir.path().join("settings.toml"));
        let writer = PrefsWriter::spawn(file.clone(), Duration::from_secs(60), |e| panic!("{e}"));
        writer.submit(with_favorite(4));
        writer.flush();
        assert!(file.load().prefs.favorites.contains(&4), "on disk when flush returns");
        drop(writer);
    }

    #[test]
    fn a_quiet_period_writes_without_waiting_for_the_drop() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file = PrefsFile::at(dir.path().join("settings.toml"));
        let writer = PrefsWriter::spawn(file.clone(), Duration::from_millis(20), |e| panic!("{e}"));
        writer.submit(with_favorite(3));
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while !file.path().exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(file.load().prefs.favorites.contains(&3));
        drop(writer);
    }

    #[test]
    fn a_failed_save_is_reported_not_swallowed() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A file where the folder should be makes every save fail.
        let blocker = dir.path().join("config");
        std::fs::write(&blocker, "x").expect("write");
        let errors: Arc<Mutex<Vec<String>>> = Arc::default();
        let sink = errors.clone();
        let writer = PrefsWriter::spawn(PrefsFile::at(blocker.join("settings.toml")), Duration::from_millis(5), move |e| {
            sink.lock().expect("lock").push(e.to_string());
        });
        writer.submit(with_favorite(1));
        drop(writer);
        assert_eq!(errors.lock().expect("lock").len(), 1);
    }
}
