//! Talks to the real internet, so it is ignored by default:
//!
//!   cargo test -p reclaw-media --test real_network -- --ignored --nocapture
//!
//! Set `SSL_CERT_FILE` to a PEM bundle if the network re-signs HTTPS traffic.
use std::time::Duration;

use reclaw_media::{DiskStore, HttpFetcher, MediaCache, Policy, Want};

fn fetcher() -> HttpFetcher {
    let fetcher = HttpFetcher::new(Duration::from_secs(30));
    match std::env::var("SSL_CERT_FILE").ok().and_then(|path| std::fs::read(path).ok()) {
        Some(pem) => fetcher.with_extra_roots(pem),
        None => fetcher,
    }
}

#[test]
#[ignore = "needs the internet"]
fn a_real_readme_and_a_real_picture_are_fetched_cached_and_read() {
    let dir = tempfile::tempdir().expect("temp dir");
    let cache = MediaCache::new(DiskStore::new(dir.path().join("media")), std::sync::Arc::new(fetcher()), Policy::default());

    let readme = cache.ensure("https://raw.githubusercontent.com/rust-lang/rust/HEAD/README.md", Want::Text).expect("the README");
    let text = std::fs::read_to_string(&readme.path).expect("readable");
    assert!(text.contains("Rust"), "{}", &text[..text.len().min(200)]);
    let document =
        reclaw_media::readme::parse(&text, &reclaw_media::readme::ReadmeContext::github("rust-lang", "rust", "HEAD").expect("valid"));
    assert!(!document.is_empty());
    println!("README: {} bytes, {} blocks", text.len(), document.blocks.len());

    let logo = cache
        .ensure("https://raw.githubusercontent.com/rust-lang/rust/HEAD/src/librustdoc/html/static/images/favicon-32x32.png", Want::Image)
        .expect("the logo");
    assert_eq!(logo.size, Some((32, 32)), "the size read from the file's header");

    // A second request is answered from disk.
    let again = cache
        .ensure("https://raw.githubusercontent.com/rust-lang/rust/HEAD/src/librustdoc/html/static/images/favicon-32x32.png", Want::Image)
        .expect("cached");
    assert_eq!(again.path, logo.path);

    // The things that must never be fetched are refused before any request.
    assert!(cache.ensure("https://127.0.0.1/a.png", Want::Image).is_err());
    assert!(cache.ensure("https://raw.githubusercontent.com/rust-lang/rust/HEAD/does-not-exist.png", Want::Image).is_err());
}
