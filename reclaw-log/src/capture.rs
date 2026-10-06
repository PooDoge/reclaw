//! One job's own share of the log, kept apart so it can be shown beside the job (an install that failed shows the lines it wrote).
//! The job runs inside [`Recording::span`]; every event logged on that thread while the span is entered, by any crate, is copied
//! into the recording as well as going to the file. The copy follows the file's level (it sits behind the same filter), and is
//! cleaned of credentials the same way.
//!
//! Why a span and not a channel the job writes to: the lines that explain a failure are mostly not the job's own. They come from the
//! network layer (a retry, a refused token, a redirect) and the unpacker, which know nothing about jobs. A span reaches them without
//! changing them.
use std::{
    collections::{HashMap, VecDeque},
    fmt::{self, Write as _},
    sync::{
        Arc, Mutex, OnceLock, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};

use tracing::{
    Event, Span, Subscriber,
    field::{Field, Visit},
    span::{Attributes, Id},
};
use tracing_subscriber::{
    Layer,
    fmt::{format::Writer, time::FormatTime},
    layer::Context,
    registry::LookupSpan,
};

use crate::redact::scrub;

/// The most lines one recording keeps; the oldest go first. A failure is explained by its last lines, and a stuck retry loop must
/// not grow a recording without bound.
pub const MAX_LINES: usize = 400;

const SPAN_NAME: &str = "recording";
const FIELD: &str = "recording";

#[derive(Default)]
struct Lines {
    kept: VecDeque<String>,
    dropped: usize,
}

type Shared = Arc<Mutex<Lines>>;

/// Recordings in progress, by key. The layer finds a span's recording here when the span is created.
fn open() -> &'static Mutex<HashMap<u64, Shared>> {
    static OPEN: OnceLock<Mutex<HashMap<u64, Shared>>> = OnceLock::new();
    OPEN.get_or_init(Default::default)
}

/// A job's lines, collected while its span is entered. Dropping it stops the collection.
pub struct Recording {
    key: u64,
    lines: Shared,
    span: Span,
}

/// Start a recording. Nothing is collected until [`Recording::span`] is entered, and nothing at all when the program's logger was
/// not installed with [`init`](crate::init) or [`init_for_tests`](crate::init_for_tests).
pub fn record() -> Recording {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let key = NEXT.fetch_add(1, Ordering::Relaxed);
    let lines = Shared::default();
    open().lock().unwrap_or_else(PoisonError::into_inner).insert(key, lines.clone());
    // At `error` so the filter never disables it: a span the filter turns off is never seen by the layer, and the recording would
    // be empty at the Problems level, which is exactly when the lines are wanted.
    let span = tracing::error_span!(target: "reclaw_log", SPAN_NAME, recording = key);
    Recording { key, lines, span }
}

impl Recording {
    /// Enter this (`span().in_scope(..)` or `let _in = span().enter()`) around the work to record.
    pub fn span(&self) -> &Span {
        &self.span
    }

    /// What was collected so far, oldest first. When lines were dropped, the first line says how many.
    pub fn lines(&self) -> Vec<String> {
        let lines = self.lines.lock().unwrap_or_else(PoisonError::into_inner);
        let note = (lines.dropped > 0).then(|| format!("({} earlier lines not kept)", lines.dropped));
        note.into_iter().chain(lines.kept.iter().cloned()).collect()
    }
}

impl Drop for Recording {
    fn drop(&mut self) {
        open().lock().unwrap_or_else(PoisonError::into_inner).remove(&self.key);
    }
}

/// Held in a recording span's extensions.
struct Sink(Shared);

/// Copies events inside a recording span into the recording. Installed by [`init`](crate::init).
pub(crate) struct CaptureLayer;

impl<S> Layer<S> for CaptureLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &Attributes<'_>, id: &Id, ctx: Context<'_, S>) {
        let meta = attrs.metadata();
        if meta.name() != SPAN_NAME || meta.target() != "reclaw_log" {
            return;
        }
        let mut key = KeyVisitor(None);
        attrs.record(&mut key);
        let Some(key) = key.0 else { return };
        let Some(shared) = open().lock().unwrap_or_else(PoisonError::into_inner).get(&key).cloned() else { return };
        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(Sink(shared));
        }
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let Some(scope) = ctx.event_scope(event) else { return };
        let Some(shared) = scope.into_iter().find_map(|span| span.extensions().get::<Sink>().map(|s| s.0.clone())) else { return };
        let line = format_line(event);
        let mut lines = shared.lock().unwrap_or_else(PoisonError::into_inner);
        if lines.kept.len() == MAX_LINES {
            lines.kept.pop_front();
            lines.dropped += 1;
        }
        lines.kept.push_back(line);
    }
}

/// `05:42:00.123  WARN reclaw_net::net: request failed host="api.github.com" attempt=2`, cleaned.
fn format_line(event: &Event<'_>) -> String {
    let mut stamp = String::new();
    // The time of day is enough beside a job that ran minutes ago; the file has the full date.
    let time = match tracing_subscriber::fmt::time::SystemTime.format_time(&mut Writer::new(&mut stamp)) {
        Ok(()) => stamp.get(11..23).unwrap_or(&stamp).to_string(),
        Err(fmt::Error) => String::new(),
    };
    let meta = event.metadata();
    let mut fields = FieldsVisitor::default();
    event.record(&mut fields);
    let mut line = format!("{time} {:>5} {}: {}", meta.level(), meta.target(), fields.message);
    for (name, value) in fields.rest {
        // Fields from the `log` bridge repeat the target and the source position.
        if !name.starts_with("log.") {
            let _ = write!(line, " {name}={value}");
        }
    }
    scrub(&line).into_owned()
}

struct KeyVisitor(Option<u64>);

impl Visit for KeyVisitor {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if field.name() == FIELD {
            self.0 = Some(value);
        }
    }

    fn record_debug(&mut self, _: &Field, _: &dyn fmt::Debug) {}
}

#[derive(Default)]
struct FieldsVisitor {
    message: String,
    rest: Vec<(&'static str, String)>,
}

impl Visit for FieldsVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.rest.push((field.name(), format!("{value:?}")));
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{value:?}");
        } else {
            self.rest.push((field.name(), format!("{value:?}")));
        }
    }
}

#[cfg(test)]
mod tests {
    use std::thread;

    use tracing_subscriber::{layer::SubscriberExt, registry};

    use super::*;

    fn with_capture<T>(f: impl FnOnce() -> T) -> T {
        tracing::subscriber::with_default(registry().with(CaptureLayer), f)
    }

    #[test]
    fn events_inside_the_span_are_kept_with_level_target_and_fields() {
        let lines = with_capture(|| {
            let recording = record();
            tracing::info!(target: "reclaw_net::net", "before");
            recording.span().in_scope(|| {
                tracing::warn!(target: "reclaw_net::net", host = "api.github.com", attempt = 2, "request failed");
                tracing::info_span!("inner").in_scope(|| tracing::info!(target: "reclaw_install", "unpacking"));
            });
            tracing::info!(target: "reclaw_net::net", "after");
            recording.lines()
        });
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].contains(" WARN reclaw_net::net: request failed host=\"api.github.com\" attempt=2"), "{lines:?}");
        assert!(lines[1].ends_with("INFO reclaw_install: unpacking"), "{lines:?}");
    }

    #[test]
    fn two_recordings_on_two_threads_keep_their_own_lines() {
        with_capture(|| {
            let dispatch = tracing::dispatcher::get_default(Clone::clone);
            let run = |name: &'static str| {
                let dispatch = dispatch.clone();
                thread::spawn(move || {
                    tracing::dispatcher::with_default(&dispatch, || {
                        let recording = record();
                        recording.span().in_scope(|| tracing::info!(target: "reclaw_app", "{name} working"));
                        recording.lines()
                    })
                })
            };
            let (a, b) = (run("a"), run("b"));
            let (a, b) = (a.join().expect("a"), b.join().expect("b"));
            assert!(a.len() == 1 && a[0].ends_with("a working"), "{a:?}");
            assert!(b.len() == 1 && b[0].ends_with("b working"), "{b:?}");
        });
    }

    #[test]
    fn a_long_recording_keeps_the_last_lines_and_says_how_many_went() {
        let lines = with_capture(|| {
            let recording = record();
            recording.span().in_scope(|| {
                for n in 0..MAX_LINES + 5 {
                    tracing::info!(target: "reclaw_app", "line {n}");
                }
            });
            recording.lines()
        });
        assert_eq!(lines.len(), MAX_LINES + 1);
        assert_eq!(lines[0], "(5 earlier lines not kept)");
        assert!(lines[1].ends_with("line 5") && lines[MAX_LINES].ends_with(&format!("line {}", MAX_LINES + 4)), "{lines:?}");
    }

    #[test]
    fn a_credential_is_removed_from_a_kept_line() {
        let lines = with_capture(|| {
            let recording = record();
            recording.span().in_scope(|| tracing::warn!(target: "reclaw_net", "refused ghp_0123456789abcdefghijklmnopqrstuvwx"));
            recording.lines()
        });
        assert!(!lines[0].contains("0123456789abcdef"), "{lines:?}");
    }

    #[test]
    fn without_the_layer_nothing_is_kept_and_nothing_breaks() {
        let recording = record();
        recording.span().in_scope(|| tracing::warn!(target: "reclaw_app", "nobody listening"));
        assert!(recording.lines().is_empty());
    }
}
