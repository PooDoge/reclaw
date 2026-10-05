//! `Net::download` against a server on this machine that cuts connections, stalls, ignores ranges and lies.
use std::{
    fs,
    io::Write,
    sync::{Arc, Mutex},
    time::Duration,
};

use reclaw_net::testing::{Reply, TestServer, local_config, local_net, pattern, sha256_hex};
use reclaw_net::{Cancel, DownloadRequest, Net, NetError, Progress};

/// A server that serves `data` the way a file host does: ranges, `ETag`, `If-Range`.
fn file_server(data: Vec<u8>, etag: &'static str) -> impl Fn(&reclaw_net::testing::Req, usize) -> Reply + Send + Sync + 'static {
    move |req, _| {
        let total = data.len();
        let range = req
            .header("range")
            .and_then(|r| r.strip_prefix("bytes="))
            .and_then(|r| r.strip_suffix('-'))
            .and_then(|n| n.parse::<usize>().ok());
        let if_range_ok = req.header("if-range").is_none_or(|v| v == etag);
        match range {
            Some(start) if if_range_ok && start < total => Reply::new(206, data[start..].to_vec())
                .header("Content-Range", &format!("bytes {start}-{}/{total}", total - 1))
                .header("ETag", etag),
            Some(start) if if_range_ok && start >= total => Reply::new(416, "").header("Content-Range", &format!("bytes */{total}")),
            _ => Reply::ok(data.clone()).header("ETag", etag),
        }
    }
}

fn no_progress(_: &Progress) {}

#[test]
fn a_file_arrives_whole_with_its_hash_and_nothing_is_left_beside_it() {
    let data = pattern(3 * 1024 * 1024 + 17);
    let server = TestServer::start(file_server(data.clone(), "\"a\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("games").join("game.zip");
    let done =
        local_net().download(&DownloadRequest::new(server.url("/game.zip"), &dest), &Cancel::new(), &mut no_progress).expect("downloaded");
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert_eq!(
        (done.bytes, done.sha256.as_str(), done.resumed_from, done.already_present),
        (data.len() as u64, sha256_hex(&data).as_str(), 0, false)
    );
    let left: Vec<_> =
        fs::read_dir(dest.parent().expect("dir")).expect("ls").flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(left, ["game.zip"], "no .part or note left behind");
    let req = &server.requests()[0];
    assert_eq!(req.header("accept-encoding"), Some("identity"), "a download is never compressed in transit");
}

#[test]
fn progress_is_reported_and_ends_at_the_total() {
    let data = pattern(2 * 1024 * 1024);
    let server = TestServer::start(file_server(data.clone(), "\"a\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let seen = Arc::new(Mutex::new(Vec::<Progress>::new()));
    let sink = seen.clone();
    local_net()
        .download(&DownloadRequest::new(server.url("/f"), dir.path().join("f")), &Cancel::new(), &mut move |p| {
            sink.lock().expect("lock").push(p.clone())
        })
        .expect("downloaded");
    let seen = seen.lock().expect("lock");
    let last = seen.last().expect("at least the final report");
    assert_eq!((last.downloaded, last.total, last.fraction()), (data.len() as u64, Some(data.len() as u64), Some(1.0)));
    assert!(seen.windows(2).all(|w| w[0].downloaded <= w[1].downloaded), "never goes backwards");
}

#[test]
fn a_connection_cut_off_halfway_continues_from_the_same_byte() {
    let data = pattern(1_000_000);
    let half = 400_000;
    let serve = file_server(data.clone(), "\"v1\"");
    let server = TestServer::start(move |req, n| {
        let mut reply = serve(req, n);
        if n == 0 {
            reply.cut_after = Some(half);
        }
        reply
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("big.bin");
    let done = local_net()
        .download(&DownloadRequest::new(server.url("/big"), &dest).sha256(&sha256_hex(&data)), &Cancel::new(), &mut no_progress)
        .expect("resumed to the end");
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert!(done.resumed_from > 0 && done.resumed_from <= half as u64, "picked up where it stopped, not at zero: {}", done.resumed_from);
    let second = &server.requests()[1];
    assert_eq!(second.header("range"), Some(format!("bytes={}-", done.resumed_from).as_str()));
    assert_eq!(second.header("if-range"), Some("\"v1\""), "guarded against the file changing in between");
}

#[test]
fn a_server_that_ignores_the_range_makes_it_start_over_correctly() {
    let data = pattern(500_000);
    let server = TestServer::start({
        let data = data.clone();
        move |_, n| {
            let mut reply = Reply::ok(data.clone()).header("ETag", "\"x\"");
            if n == 0 {
                reply.cut_after = Some(100_000);
            }
            reply
        }
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    let done = local_net().download(&DownloadRequest::new(server.url("/f"), &dest), &Cancel::new(), &mut no_progress).expect("downloaded");
    assert_eq!(fs::read(&dest).expect("file"), data, "not 100 kB of the old attempt followed by the whole file");
    assert_eq!(done.resumed_from, 0);
}

#[test]
fn a_file_that_changed_between_attempts_is_fetched_afresh_not_stitched() {
    let (old, new) = (pattern(600_000), pattern(600_000).into_iter().rev().collect::<Vec<u8>>());
    let (old2, new2) = (old.clone(), new.clone());
    let server = TestServer::start(move |req, n| {
        if n == 0 {
            let mut r = Reply::ok(old2.clone()).header("ETag", "\"old\"");
            r.cut_after = Some(200_000);
            r
        } else {
            // The file has been replaced: If-Range no longer matches, so the whole new file is the answer.
            file_server(new2.clone(), "\"new\"")(req, n)
        }
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    let done = local_net().download(&DownloadRequest::new(server.url("/f"), &dest), &Cancel::new(), &mut no_progress).expect("downloaded");
    assert_eq!(fs::read(&dest).expect("file"), new);
    assert_ne!(fs::read(&dest).expect("file"), old);
    assert_eq!(done.resumed_from, 0);
}

#[test]
fn a_file_with_the_wrong_hash_is_rejected_and_deleted() {
    let data = pattern(10_000);
    let server = TestServer::start(file_server(data, "\"a\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    let err = local_net()
        .download(&DownloadRequest::new(server.url("/f"), &dest).sha256("sha256:00ff"), &Cancel::new(), &mut no_progress)
        .expect_err("mismatch");
    assert!(matches!(err, NetError::Integrity(ref m) if m.contains("SHA-256")), "{err:?}");
    assert!(!dest.exists(), "a file that is not what was promised is never put in place");
    assert_eq!(fs::read_dir(dir.path()).expect("ls").count(), 0, "and no scratch files either");
}

#[test]
fn a_wrong_size_is_caught_before_a_byte_is_written() {
    let server = TestServer::start(file_server(pattern(10_000), "\"a\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let err = local_net()
        .download(&DownloadRequest::new(server.url("/f"), dir.path().join("f")).size(9_999), &Cancel::new(), &mut no_progress)
        .expect_err("size");
    assert!(matches!(err, NetError::Integrity(_)), "{err:?}");
    let too_big = local_net().download(
        &DownloadRequest { max_bytes: 100, ..DownloadRequest::new(server.url("/f"), dir.path().join("g")) },
        &Cancel::new(),
        &mut no_progress,
    );
    assert_eq!(too_big, Err(NetError::TooLarge { limit: 100 }));
    assert_eq!(fs::read_dir(dir.path()).expect("ls").count(), 0);
}

#[test]
fn a_file_already_in_place_with_the_right_hash_is_not_fetched_again() {
    let data = pattern(50_000);
    let server = TestServer::start(file_server(data.clone(), "\"a\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    fs::write(&dest, &data).expect("write");
    let done = local_net()
        .download(&DownloadRequest::new(server.url("/f"), &dest).sha256(&sha256_hex(&data)), &Cancel::new(), &mut no_progress)
        .expect("present");
    assert!(done.already_present);
    assert_eq!(server.count(), 0, "not a single request");
    // The wrong file in place is replaced.
    fs::write(&dest, b"stale").expect("write");
    let again = local_net()
        .download(&DownloadRequest::new(server.url("/f"), &dest).sha256(&sha256_hex(&data)), &Cancel::new(), &mut no_progress)
        .expect("replaced");
    assert!(!again.already_present);
    assert_eq!(fs::read(&dest).expect("file"), data);
}

#[test]
fn cancelling_keeps_the_partial_file_and_a_later_call_finishes_it() {
    let data = pattern(800_000);
    let serve = file_server(data.clone(), "\"c\"");
    let server = TestServer::start(move |req, n| {
        let mut reply = serve(req, n);
        if n == 0 {
            // Half the file, then silence: the user gives up while it hangs.
            reply.stall_after = Some((300_000, Duration::from_secs(3)));
        }
        reply
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    let cancel = Cancel::new();
    let trigger = cancel.clone();
    let err = local_net()
        .download(&DownloadRequest::new(server.url("/f"), &dest), &cancel, &mut move |p| {
            if p.downloaded > 0 {
                trigger.cancel()
            }
        })
        .expect_err("cancelled");
    assert_eq!(err, NetError::Cancelled);
    let kept = fs::metadata(dir.path().join("f.part")).expect("partial kept").len();
    assert!(kept > 0 && kept < data.len() as u64, "{kept}");
    assert!(!dest.exists());
    // Later, the same download resumes instead of starting again.
    let done = local_net().download(&DownloadRequest::new(server.url("/f"), &dest), &Cancel::new(), &mut no_progress).expect("finished");
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert_eq!(done.resumed_from, kept);
}

#[test]
fn a_server_that_goes_quiet_is_noticed_and_the_transfer_resumes() {
    let data = pattern(600_000);
    let serve = file_server(data.clone(), "\"s\"");
    let server = TestServer::start(move |req, n| {
        let mut reply = serve(req, n);
        if n == 0 {
            reply.stall_after = Some((250_000, Duration::from_secs(10)));
        }
        reply
    });
    let mut config = local_config();
    config.stall_timeout = Duration::from_millis(600);
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    let started = std::time::Instant::now();
    let done = Net::new(config)
        .expect("net")
        .download(&DownloadRequest::new(server.url("/f"), &dest), &Cancel::new(), &mut no_progress)
        .expect("recovered");
    assert!(started.elapsed() < Duration::from_secs(8), "did not wait out the stall: {:?}", started.elapsed());
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert!(done.resumed_from > 0);
}

#[test]
fn a_partial_file_that_is_in_fact_complete_is_finished_by_the_416_answer() {
    let data = pattern(100_000);
    let server = TestServer::start(file_server(data.clone(), "\"z\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    fs::write(dir.path().join("f.part"), &data).expect("part");
    let note = format!(r#"{{"url":"{}","etag":"\"z\"","last_modified":null,"total":{}}}"#, server.url("/f"), data.len());
    fs::write(dir.path().join("f.part.json"), note).expect("note");
    let done = local_net()
        .download(&DownloadRequest::new(server.url("/f"), &dest).sha256(&sha256_hex(&data)), &Cancel::new(), &mut no_progress)
        .expect("finished");
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert_eq!(done.bytes, data.len() as u64);
}

#[test]
fn a_leftover_partial_for_another_address_is_not_resumed() {
    let data = pattern(100_000);
    let server = TestServer::start(file_server(data.clone(), "\"z\""));
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f");
    fs::write(dir.path().join("f.part"), b"bytes of something else").expect("part");
    fs::write(dir.path().join("f.part.json"), r#"{"url":"http://elsewhere.test/other","etag":null,"last_modified":null,"total":23}"#)
        .expect("note");
    let done = local_net().download(&DownloadRequest::new(server.url("/f"), &dest), &Cancel::new(), &mut no_progress).expect("downloaded");
    assert_eq!(fs::read(&dest).expect("file"), data);
    assert_eq!(done.resumed_from, 0);
    assert!(server.requests()[0].header("range").is_none());
}

#[test]
fn an_archive_is_never_unpacked_in_transit_even_if_the_server_labels_it_gzip() {
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(&pattern(20_000)).expect("gzip");
    let packed = encoder.finish().expect("finish");
    let server = TestServer::start({
        let packed = packed.clone();
        move |_, _| Reply::ok(packed.clone()).header("Content-Encoding", "gzip")
    });
    let dir = tempfile::tempdir().expect("tempdir");
    let dest = dir.path().join("f.tar.gz");
    local_net().download(&DownloadRequest::new(server.url("/f.tar.gz"), &dest), &Cancel::new(), &mut no_progress).expect("downloaded");
    assert_eq!(fs::read(&dest).expect("file"), packed, "the bytes are the file, not what the header claimed");
}

#[test]
fn errors_from_the_server_are_classified_like_any_other_request() {
    let server = TestServer::start(|_, _| Reply::new(404, "gone"));
    let dir = tempfile::tempdir().expect("tempdir");
    let err = local_net()
        .download(&DownloadRequest::new(server.url("/f"), dir.path().join("f")), &Cancel::new(), &mut no_progress)
        .expect_err("404");
    assert!(matches!(err, NetError::Status { status: 404, .. }), "{err:?}");
    assert_eq!(server.count(), 1);
    let limited = TestServer::start(|_, _| Reply::new(429, "").header("Retry-After", "300"));
    let err = local_net()
        .download(&DownloadRequest::new(limited.url("/f"), dir.path().join("g")), &Cancel::new(), &mut no_progress)
        .expect_err("429");
    assert!(matches!(err, NetError::RateLimited { .. }), "{err:?}");
}
