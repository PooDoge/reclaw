# 0011 One async HTTP client that says who it is

- status: accepted
- date: 2026-10-05
- spec: ../specs/network.md

## Context

The launcher fetches a catalog, release lists, installers of hundreds of megabytes, artwork and (later) mod listings. The first
attempt at "loading real data" met errors that were described as Content Security Policy errors, with the suggestion to get past them
with a proxy or a spoofed user agent. Probing every host the program uses, from the machine that had the problem and with an
honest user agent (`Reclaw/0.1 (+https://github.com/poodoge/reclaw)`), on 2026-10-05:

* `raw.githubusercontent.com` answered 200 and sends `content-security-policy: default-src 'none'; sandbox`. `api.github.com` and
  `gitlab.com` also send one. A CSP is a response header that **a browser** enforces on a page; a program reads it as text and nothing
  happens. It cannot be the reason a request failed, and no user agent or proxy changes it.
* `gitlab.com` answered 200 to four different user agents (this program's, `QuiverLauncher/1.0`, `curl/8.5.0` and a Chrome string) for
  the same three addresses. The one 403 seen earlier was a different project, whose releases need a login.
* `thunderstore.io`, `gamebanana.com`, `cdn2.steamgriddb.com`: the **proxy refused the CONNECT** (403) before any request existed. That
  is the sandbox's egress policy, which says to report the host, not to route around it.
* `api.github.com` and `github.com` for repositories other than the one attached to the session: a 403 whose body says "GitHub access to
  this repository is not enabled for this session". Also policy.
* Unauthenticated GitHub API use is limited to 60 requests an hour; a conditional request that returns 304 does not count against it.
  Release assets carry a SHA-256 `digest` since June 2025.

So the failures that exist are policy refusals, rate limits and (possibly, on a user's own network, not yet seen) bot-check pages,
and they look alike to a client that only reads status codes.

## Decision

`reclaw-net`: **asynchronous `reqwest` 0.12 with rustls, on a small private tokio runtime**, behind a synchronous API (a worker thread
calls `fetch` or `download` and waits). It identifies itself honestly on every request. It never imitates a browser: a failure is
classified (`NetError`: refused by the network's policy, bot check, rate limit, certificate, DNS ...) and shown with a hint, and the
person decides what to do (allow the host, add a token, use a mirror). It reads `HTTPS_PROXY`/`NO_PROXY`, `SSL_CERT_FILE`, `RECLAW_PROXY`
and `GITHUB_TOKEN`/`GITLAB_TOKEN` (sent only to that service's own host and dropped on a cross-host redirect). `Net::probe` and the
`probe` example report what each host does from the machine they run on.

Fast and efficient, concretely: one pooled client so many small requests share a connection (HTTP/2); gzip and brotli for JSON;
`ETag` and `Last-Modified` revalidation through an on-disk cache that can serve a stale copy when the network is down; per-host
concurrency limits and a memory of "this host asked me to wait" so a library of 200 apps does not ask GitHub 200 questions at once;
downloads streamed to disk (never held in memory), hashed as they arrive, resumed with `Range` guarded by `If-Range` after a cut or a
stall or a cancel, and skipped altogether when a file with the right SHA-256 is already in place.

## Rejected

* **Spoofing a browser, or a proxy to get around a refusal.** Probing showed the user agent changes nothing where it was tested, and the
  refusals that exist are policy, which a disguise would defeat only by breaking the rule it enforces. It would also make the program
  worse for the sites it depends on: GitHub asks for an identifying user agent, and a launcher that hides what it is is the first thing a
  rate limiter blocks. If a site is found to block honest clients, the answer is its API with a token, a mirror, or asking its
  maintainers, in that order.
* **`wreq` / `rquest` (TLS and HTTP/2 fingerprint emulation).** Their purpose is to pass bot detection. They build BoringSSL (a C++
  toolchain on every contributor's machine, including a distrobox on an immutable system), duplicate the TLS stack Freya already links,
  and the evidence above says they would buy nothing here.
* **`reqwest::blocking`** (what `reclaw-media` used). It has no per-read timeout, so a download cannot tell a slow server from a dead one,
  and cancelling a transfer means abandoning a thread.
* **`ureq`.** Small and synchronous, but HTTP/1.1 only: no multiplexing for the many small catalog requests.
* **`reqwest-middleware` plus `http-cache-reqwest` and `reqwest-retry`.** The cache middleware is at an alpha version that needs a recent
  compiler, and the rules we need (a time to live per endpoint, stale on error, separate copies per credential) are narrower than a
  general HTTP cache. About 200 lines here, tested against a real socket.
* **libcurl bindings.** A C library and its certificates as another thing to build and trust.
* **Several connections for one download.** It can raise throughput on some servers and can look like abuse to others; the one host that
  matters (GitHub's release CDN) cannot be reached from where this was built, so the gain is unmeasured. A single resumable stream first;
  segmenting is a change inside `download` if a measurement says it pays.
* **`hickory-dns`.** The system resolver is what the user's network is configured for.

## Consequences

`reclaw-media` fetches through the same client (its `HttpFetcher` is now a thin adapter), so pictures, READMEs, the catalog and
installers share one set of connections, one proxy setting and one certificate store. The blocking facade must not be called from async
code; it returns `WrongContext` instead of panicking. The Windows and macOS proxy and certificate paths are reqwest's own and were not
exercised. Sixty requests an hour is too few for 232 projects without a token: the platform index (one file, one request) is how the
catalog gets every project's latest release without asking the API about each.
