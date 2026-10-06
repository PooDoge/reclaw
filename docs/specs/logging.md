# Logging

- last-verified: 2026-10-06
- owner-paths: reclaw-log/src/**, reclaw-log/tests/**, reclaw-app/src/main.rs, reclaw-ui/src/notices/log.rs, reclaw-app/src/host/report.rs

The program writes everything that went wrong, and enough around it to explain why, to a file (ADR 0013). A person helping with a
problem asks for `reclaw.log`, or for the diagnostics report, and needs nothing else.

## Where

* `reclaw.log` in the logs folder: `~/.local/state/reclaw/logs` on Linux (XDG state), the local data folder's `logs` elsewhere,
  `$RECLAW_HOME/logs` when `RECLAW_HOME` is set. Settings, Diagnostics, "Open the log folder" shows it.
* Older runs are `reclaw.1.log` ... `reclaw.4.log`. A file is moved aside at 2 MiB; at most about 10 MiB exist.
* The terminal as well, when standard error is one (`cargo run`).
* Each run begins with a line naming the version, the commit, the profile, the system, and the log file.

## What a line looks like

`2026-10-05T05:49:07.476Z  WARN reclaw-catalog reclaw_net::net: the network (a proxy or firewall) refused this host; not asking again
for a while host=thunderstore.io minutes=5` : time (UTC), level, thread, module, message, then `field=value` facts.

## What is written

* **Our crates at `info`**, everything else at `warn`: a catalog refresh and its result, a token saved or checked, a download's start and
  finish, a setting that could not be saved, the host's answers to effects (the name, not the contents).
* **`debug`** (Settings: Detailed): every request (`fetched url=host/path status=200 bytes=997 source=CacheFresh ms=0`), every retry with its
  attempt and wait, each token change, the saved catalog read at start.
* **Every notice the user is shown**, with its details, from `Store::dispatch` (`reclaw_ui::notices::log`): `problem shown to the user
  title=... body=... details=...`. What was on screen and what is in the log are the same set.
* **A panic**: the thread, the place, the message and the stack, then the usual message.
* **One job's own lines, also kept beside the job** (`reclaw_log::record`, ADR 0019): the install worker runs inside a recording span, and
  every event on that thread while it is entered, from any crate (the network layer's retries, the unpacker), is copied into the
  recording as well as the file: same level, same redaction, the time of day instead of the date, the last 400 lines (the first line
  then says how many earlier ones went). A failed install sends them with its failure, and the "Failed" label shows them. Each job's
  file lines carry `recording{recording=N}` so they can be found together.
* The toolkit's and the HTTP stack's own warnings (no Vulkan driver, a compositor reporting a 0 mm display) arrive through the same
  subscriber and are often the answer.

URLs are written as `host/path`: never the query, which is where a credential would be.

## Levels

Settings, Diagnostics, Log detail: **Problems only** (our warnings, others' errors), **Normal** (default), **Detailed** (our debug,
others' info). Changed while running. `RECLAW_LOG=<filter>` (`reclaw_net=trace,warn`) replaces it and then Settings does not change it.

## What is never written

Credentials, in any form. Three layers (`reclaw-log`): the `Secret` type prints as `‹redacted›`; every token read from the tokens file,
typed, or taken from the environment is registered, and its exact text is removed from every line; token-shaped text (`ghp_...`,
`github_pat_...`, `glpat-...`), `Authorization`-style headers, `user:password@` in an address and `?access_token=` values are removed
by pattern. An event is cleaned whole, so a token split across two writes is still removed. `tests/repo_hygiene.rs` fails if a
program crate prints with `println!`/`eprintln!` instead of logging, if a crate is missing from `OUR_TARGETS`, or if code outside the
files that must hold a token reads one's text (`.expose()`).

## Diagnostics report

Settings, Diagnostics, "Save a diagnostics report" writes `diagnostics-<time>.txt` beside the log: versions and commit; the system,
distribution, kernel and session; the proxy and display variables; where each folder is; whether each service has a token and what it
allows; what every host the program needs answers from this machine (asked in parallel); the catalog's state; and the last 200 log
lines. It passes through the same redaction.

## Tests

`reclaw-log` has unit tests (rotation, redaction, level mapping, tail, recording: span scope, two threads, the line cap, redaction) and one integration test installing the real logger (file
contents, level change in both directions, a panic on another thread, a second `init`). `reclaw-net/tests/logging.rs` checks the
request trail; the host and UI tests check that notices and statuses carry no token. Other tests call `reclaw_log::init_for_tests()` to
see what the code logged when a test fails.

## Not verified

The log folder on Windows and macOS; behaviour when the logs folder is on a full disk (the write error goes to the terminal once and
the program continues); two copies of the program writing the same file (appends are whole lines, rotation may then collide).
