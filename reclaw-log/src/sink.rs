//! Where formatted events go, with credentials taken out on the way. An event is collected whole and cleaned as one piece, so a
//! token cannot slip through by being split across two writes.
use std::{
    io::{self, Write},
    sync::{Arc, Mutex, PoisonError},
};

use tracing_subscriber::fmt::MakeWriter;

use crate::{redact::scrub, rotate::RotatingFile};

/// A destination for finished lines.
pub(crate) trait Dest {
    fn put(&self, text: &str);
}

/// The log file, shared by every thread.
#[derive(Clone)]
pub(crate) struct FileDest(pub(crate) Arc<Mutex<RotatingFile>>);

impl Dest for FileDest {
    fn put(&self, text: &str) {
        let mut file = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        // A log that cannot be written must not take the program down, and there is nowhere left to say so but stderr.
        if let Err(error) = file.write_all(text.as_bytes()) {
            eprintln!("reclaw: could not write to the log file: {error}");
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct StderrDest;

impl Dest for StderrDest {
    fn put(&self, text: &str) {
        // Same reasoning: a closed stderr (a desktop entry) is normal, not an error to report.
        let _ = io::stderr().write_all(text.as_bytes());
    }
}

/// One event being written.
pub(crate) struct EventWriter<'a, D: Dest> {
    dest: &'a D,
    buf: Vec<u8>,
}

impl<D: Dest> EventWriter<'_, D> {
    fn emit(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = String::from_utf8_lossy(&self.buf);
        self.dest.put(&scrub(&text));
        self.buf.clear();
    }
}

impl<D: Dest> Write for EventWriter<'_, D> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.emit();
        Ok(())
    }
}

impl<D: Dest> Drop for EventWriter<'_, D> {
    fn drop(&mut self) {
        self.emit();
    }
}

impl<'a> MakeWriter<'a> for FileDest {
    type Writer = EventWriter<'a, FileDest>;

    fn make_writer(&'a self) -> Self::Writer {
        EventWriter { dest: self, buf: Vec::new() }
    }
}

impl<'a> MakeWriter<'a> for StderrDest {
    type Writer = EventWriter<'a, StderrDest>;

    fn make_writer(&'a self) -> Self::Writer {
        EventWriter { dest: self, buf: Vec::new() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Collect(Mutex<Vec<String>>);

    impl Dest for Collect {
        fn put(&self, text: &str) {
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(text.to_string());
        }
    }

    #[test]
    fn a_token_split_across_writes_is_still_removed() {
        let dest = Collect::default();
        {
            let mut writer = EventWriter { dest: &dest, buf: Vec::new() };
            writer.write_all(b"failed with ghp_0123456789").expect("write");
            writer.write_all(b"abcdefghijklmnop and went on\n").expect("write");
        }
        let lines = dest.0.lock().expect("lock").clone();
        assert_eq!(lines.len(), 1, "one event, one write to the destination");
        assert!(!lines[0].contains("0123456789abcdef"), "{}", lines[0]);
        assert!(lines[0].ends_with("went on\n"));
    }

    #[test]
    fn an_empty_event_writes_nothing() {
        let dest = Collect::default();
        drop(EventWriter { dest: &dest, buf: Vec::new() });
        assert!(dest.0.lock().expect("lock").is_empty());
    }
}
