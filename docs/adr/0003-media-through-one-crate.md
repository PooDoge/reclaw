# 0003 Everything fetched from the internet goes through reclaw-media

- status: accepted
- freya: claims about the toolkit checked against 0.5.0-rc.8 (tag v0.5.0-rc.8, commit af55a77) on 2026-10-04
- date: 2026-10-04
- spec: ../specs/media.md

## Context

Artwork, screenshots and READMEs come from the internet, written by strangers: a README is arbitrary markdown and HTML with images the
author chose. The UI toolkit can fetch images itself (its `remote-asset` feature) and has an HTML renderer that fetches too. Neither
has a size limit, an address policy, or a disk cache.

## Decision

A Freya-free crate, `reclaw-media`, is the only code that fetches: https only, standard port, nothing on the local network (also
after redirects), size and time caps, file kind decided from the bytes, an atomic on-disk cache with freshness, a stale fallback and
failure memory, and a small worker pool that joins duplicate requests. The toolkit's `remote-asset` and `html` features are
switched off, so nothing else can fetch.

## Rejected

* **The toolkit's remote images.** Convenient (`ImageViewer` takes a URL) but a README image could reach `http://192.168.0.1/...`, be
  any size, and be fetched again on every launch.
* **Fetching in the UI crate.** It would tie the safety rules to a window, and they could not be tested against a fake network.
* **Trusting `Content-Type`.** GitHub serves some raw files as `text/plain`, and a hostile server can say anything.
* **A cache keyed by `DefaultHasher`.** Its output is not promised stable across Rust versions, which would orphan the cache. Names
  use FNV-1a, and the description file records whose the data is, so a name clash reads as a miss.
* **No cache, only the toolkit's in-memory one.** Every launch would download every image again.

## Consequences

Two crates carry an HTTP client (the toolkit's is gone, ours uses the same `reqwest` build). The UI never sees a URL it can load by
itself: it asks the hub and gets a file path. A network that re-signs HTTPS needs `SSL_CERT_FILE`. DNS rebinding is not defended.
