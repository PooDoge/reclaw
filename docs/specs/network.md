# Network: the one HTTP client

- last-verified: 2026-10-05
- owner-paths: reclaw-net/src/**, reclaw-net/tests/**, reclaw-net/examples/probe.rs, reclaw-media/src/fetch.rs

Everything the program fetches goes through `reclaw_net::Net` (ADR 0010, 0011). `reclaw-media` adds picture-and-README rules on top
(`media.md`). Call from a worker thread; from async code every method returns `NetError::WrongContext`.

## What a request does

* **Identifies itself** as `Reclaw/<version> (+https://github.com/poodoge/reclaw)` and never as a browser.
* **Address policy** before anything is sent and on every redirect: https, port 443, no credentials, nothing on the local network
  (`address.rs`). Tests use `AddressPolicy::AnyHttpForTests`; nothing in the program selects it.
* **Proxy and certificates**: the system's proxy settings (`HTTPS_PROXY`, `NO_PROXY`; `RECLAW_PROXY=none|<url>` overrides); the system's
  trust store plus the bundled public roots; `SSL_CERT_FILE` adds a PEM bundle.
* **Tokens** (`GITHUB_TOKEN`, `GITLAB_TOKEN`, or the `RECLAW_` forms) are sent as `Authorization: Bearer` to `api.github.com` and
  `gitlab.com` only, are marked sensitive, are dropped when a redirect leaves the host, never appear in `Debug` output, and separate the
  cache (an authenticated answer is never served to an anonymous request).
* **Retries** (default 3 tries) with 0.5 s doubling to 8 s, jittered, for connection failures, timeouts, 408/425/500/502/503/504. A 404
  or 403 is an answer and is not retried. Policy refusals and bot checks are not retried.
* **Rate limits** are read from `Retry-After`, `x-ratelimit-*` and `ratelimit-*`. A wait up to 5 s is slept through; a longer one is
  returned as `RateLimited` and the host is remembered as blocked, so the next request fails at once without being sent.
* **Concurrency**: 6 requests in flight per host.
* **Small answers** (`fetch`): body capped as it arrives; optional on-disk cache with a time to live, `If-None-Match` /
  `If-Modified-Since` revalidation, and an old copy returned (marked `CacheStale`, with the reason) when the network is unwell and the
  request allows it.

## Downloads

`Net::download` streams to `<dest>.part` with `<dest>.part.json` (address, `ETag`, total), hashes as it goes, and moves the finished file
into place only after the size and SHA-256 (`sha256:<hex>` as GitHub writes it) match. A cut connection, a stall (no data for 20 s) or
a cancel keeps the partial file; the next attempt, in the same call or a later one, asks for the rest with `Range` and `If-Range`. A
server that ignores the range, or a file that changed, restarts from zero. A file already in place with the right hash is not fetched.
Compression is never negotiated for downloads. A wrong hash or size deletes everything and is an error.

## Failure kinds

`NetError` separates: refused by the address policy; DNS; connect; **refused by a proxy or the network's policy** (`ProxyDenied`);
certificate (`Tls`, hint: `SSL_CERT_FILE`); timeout and stall; HTTP status; **rate limited** (with the wait); **bot check**
(`cf-mitigated: challenge`, or an HTML interstitial on 401/403/429/503); too large; cancelled; disk full; integrity. Each that a person
can act on has a `hint()`.

## Content Security Policy

A CSP is a header a browser enforces on a page. This client reads it as text and nothing happens; the `probe` example prints it so the
point is visible. It is never a reason a request fails here.

## Seeing it on your machine

```sh
cargo run -p reclaw-net --example probe            # every host the program uses
cargo run -p reclaw-net --example probe -- URL...  # your own
```

From the build sandbox on 2026-10-05 (honest user agent): the catalog files, GitLab's API and raw files answered; `thunderstore.io`,
`gamebanana.com` and `cdn2.steamgriddb.com` were refused by the sandbox's proxy policy; `github.com` pages and `api.github.com` for
repositories not attached to the session were refused by its GitHub gateway.

## Not verified

Downloading a real release asset (the host that serves them was unreachable from the build sandbox), HTTP/2 throughput on very large
files, the Windows and macOS proxy and certificate paths, any bot check on a user's network (none was seen), the mod sites' real
responses.
