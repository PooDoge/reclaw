//! The logger as a program uses it: installed once, writing a file, never writing a credential, following the level, recording a
//! panic. One test function, because the logger is process-wide.
use std::{fs, thread};

use reclaw_log::{LogConfig, LogLevel, init, register_secret};

#[test]
fn the_installed_logger_writes_cleans_follows_the_level_and_records_a_panic() {
    let dir = tempfile::tempdir().expect("tempdir");
    let logs = dir.path().join("logs");
    let logging = init(LogConfig { dir: Some(logs.clone()), level: LogLevel::Normal, filter: None, stderr: false });
    assert!(logging.problems().is_empty(), "{:?}", logging.problems());
    let file = logging.file().expect("a file").to_path_buf();
    assert_eq!(file, logs.join("reclaw.log"));
    let read = || fs::read_to_string(&file).expect("read the log");

    // Our info is written, with level, target and thread; our debug is not.
    tracing::info!(target: "reclaw_net::net", host = "api.github.com", attempt = 2, "fetched");
    tracing::debug!(target: "reclaw_net::net", "too detailed for Normal");
    let text = read();
    assert!(
        text.contains("INFO") && text.contains("reclaw_net::net") && text.contains("host=\"api.github.com\"") && text.contains("attempt=2"),
        "{text}"
    );
    assert!(!text.contains("too detailed"), "{text}");

    // A dependency's info is not; its warning is.
    tracing::info!(target: "hyper_util::client", "connection pooled");
    tracing::warn!(target: "hyper_util::client", "connection reset");
    let text = read();
    assert!(!text.contains("connection pooled") && text.contains("connection reset"), "{text}");

    // Credentials do not get in, by shape or by registration, in the message or in a field.
    register_secret("corp-issued-credential-99");
    tracing::warn!(target: "reclaw_net", "bad credentials ghp_0123456789abcdefghijklmnopqrstuvwx for corp-issued-credential-99");
    tracing::warn!(target: "reclaw_net", url = "https://u:pw@example.org/f?access_token=abcdef123", "request failed");
    let text = read();
    for leaked in ["ghp_0123456789", "corp-issued", "pw@", "abcdef123"] {
        assert!(!text.contains(leaked), "{leaked} leaked:\n{text}");
    }
    assert!(text.contains("‹redacted›"));

    // The level changes while running, in both directions.
    logging.set_level(LogLevel::Detailed);
    tracing::debug!(target: "reclaw_net::net", "now visible");
    assert!(read().contains("now visible"));
    logging.set_level(LogLevel::Problems);
    tracing::info!(target: "reclaw_net::net", "now hidden");
    tracing::error!(target: "reclaw_net::net", "still visible");
    let text = read();
    assert!(!text.contains("now hidden") && text.contains("still visible"), "{text}");

    // A panic on another thread is recorded with its place, thread and stack.
    let joined = thread::Builder::new()
        .name("worker-under-test".into())
        .spawn(|| panic!("the catalog index was not an object"))
        .expect("spawn")
        .join();
    assert!(joined.is_err());
    let text = read();
    assert!(text.contains("panic: the catalog index was not an object"), "{text}");
    assert!(text.contains("worker-under-test") && text.contains("tests/logging.rs"), "{text}");
    // The stack is captured whatever RUST_BACKTRACE says: numbered frames, with the place of the panic among them.
    assert!(text.contains("   0: ") && text.contains("at ./tests/logging.rs:"), "the frames follow: {text}");

    // A second logger is reported, not a crash.
    let again = init(LogConfig { dir: None, level: LogLevel::Normal, filter: None, stderr: false });
    assert!(again.problems().iter().any(|p| p.contains("already installed")), "{:?}", again.problems());
}
