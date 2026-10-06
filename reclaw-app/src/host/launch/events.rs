//! What the supervisor says about the apps it runs, carried to the screens: a session began, a session ended and how.
use std::{
    future::Future,
    pin::pin,
    sync::{Arc, Weak},
    task::{Context, Poll, Wake, Waker},
    thread,
    time::{Duration, SystemTime},
};

use futures_util::StreamExt;
use reclaw_runtime::{Outcome, RunState, SessionEvent};
use reclaw_ui::{notices::Notice, store::AppAction};

use crate::host::{Host, Inner};

/// Wakes the thread that is waiting on a future.
struct Unpark(thread::Thread);

impl Wake for Unpark {
    fn wake(self: Arc<Self>) {
        self.0.unpark();
    }
}

/// Run a future to its end on this thread, sleeping between its wake-ups. (The one future here is the supervisor's channel.)
fn block_on<F: Future>(future: F) -> F::Output {
    let mut future = pin!(future);
    let waker = Waker::from(Arc::new(Unpark(thread::current())));
    let mut cx = Context::from_waker(&waker);
    loop {
        match future.as_mut().poll(&mut cx) {
            Poll::Ready(value) => return value,
            Poll::Pending => thread::park(),
        }
    }
}

impl Host {
    /// Carry the supervisor's events to the screens until the host is gone. Holds the host only weakly, so it does not keep it alive.
    pub(in crate::host) fn watch_sessions(&self, mut events: futures_channel::mpsc::UnboundedReceiver<SessionEvent>) {
        let weak: Weak<Inner> = Arc::downgrade(&self.inner);
        let started = thread::Builder::new().name("reclaw-sessions".into()).spawn(move || {
            while let Some(event) = block_on(events.next()) {
                let Some(inner) = weak.upgrade() else { return };
                Host { inner }.on_session(&event);
            }
        });
        if let Err(error) = started {
            tracing::error!(%error, "the thread that follows running apps did not start; their state will not be shown");
        }
    }

    fn on_session(&self, event: &SessionEvent) {
        match event {
            SessionEvent::Started { app, pid } => {
                tracing::info!(app, pid, "an app started");
                self.send(AppAction::SetRun { id: *app, run: RunState::Running { pid: *pid, since: SystemTime::now() } });
            }
            SessionEvent::Ended { app, outcome } => {
                tracing::info!(app, ?outcome, "an app ended");
                let run = if outcome.is_failure() { RunState::Failed(outcome.clone()) } else { RunState::Idle };
                self.send(AppAction::SetRun { id: *app, run });
                if let Some(notice) = self.ended_notice(*app, outcome) {
                    self.tell(notice);
                }
            }
        }
    }

    /// Something to say about how a session ended, or nothing when it ended the way a game ends.
    fn ended_notice(&self, app: u32, outcome: &Outcome) -> Option<Notice> {
        let title = self.title_of(app).unwrap_or_else(|| "The app".to_string());
        let log = self.game_log(app).map(|p| format!("Its output is in {}", p.display()));
        let quick = outcome.ran_for() < Duration::from_secs(5);
        let (headline, body, mut details) = match outcome {
            Outcome::Signaled { signal, .. } => (
                format!("{title} crashed"),
                format!("It was ended by signal {signal} after {}", ran(outcome)),
                vec![format!("Signal {signal} is {}", signal_name(*signal))],
            ),
            Outcome::ExitedWithCode { code, .. } if quick => (
                format!("{title} closed right after starting"),
                format!("It exited with code {code} after {}", ran(outcome)),
                vec!["It may be missing a file or a library, or be the wrong build for this system.".to_string()],
            ),
            // A non-zero exit later on is how many games quit; the Play button says Retry and that is enough.
            _ => return None,
        };
        details.extend(log);
        Some(Notice::problem(&headline, &body, details))
    }
}

fn ran(outcome: &Outcome) -> String {
    let secs = outcome.ran_for().as_secs();
    if secs < 60 { format!("{secs} s") } else { format!("{} min", secs / 60) }
}

/// The usual names of the signals that end a program badly.
fn signal_name(signal: i32) -> &'static str {
    match signal {
        4 => "an illegal instruction (the program needs a CPU feature this one lacks)",
        6 => "an abort (the program stopped itself after an error)",
        7 | 10 => "a bus error",
        8 => "an arithmetic error",
        9 => "a kill (it was ended from outside)",
        11 => "a segmentation fault (the program crashed)",
        15 => "a request to end",
        _ => "a signal that ends a program",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signals_have_plain_names() {
        assert!(signal_name(11).contains("crashed"));
        assert!(signal_name(4).contains("CPU feature"));
        assert!(signal_name(99).contains("signal"));
    }

    #[test]
    fn a_future_that_is_already_done_is_returned_and_one_that_waits_is_woken() {
        assert_eq!(block_on(async { 7 }), 7);
        let (tx, mut rx) = futures_channel::mpsc::unbounded::<u32>();
        let sender = thread::spawn(move || {
            thread::sleep(Duration::from_millis(30));
            let _ = tx.unbounded_send(5);
        });
        assert_eq!(block_on(rx.next()), Some(5));
        sender.join().expect("sender");
        assert_eq!(block_on(rx.next()), None, "the channel closed with the sender");
    }
}
