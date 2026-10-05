# 0013 The log is a file, written from the first line

- status: accepted
- date: 2026-10-05
- spec: ../specs/logging.md

## Context

Failures were being chased without context. The program printed with `eprintln!` in nine places, and a launcher is started from a menu
entry, where standard error goes nowhere. A failure that is only printed is a failure nobody can look at afterwards, and the person
who sees a red notice and the person who has to fix it are usually not in the same place. What is needed is the history *before* anyone
knows something is wrong: which request, to which host, which attempt, what the server said, how long it took, what the user was shown.

The toolkit already emits through `tracing` (it is in the dependency tree), so its warnings (no Vulkan driver, a compositor that
reports a 0 mm screen) are exactly the facts needed to debug a Bazzite report and were being dropped too.

## Decision

* Every crate writes with the `tracing` macros and knows nothing about files. The binary installs one subscriber, first thing, in
  `reclaw-log` (a crate of its own with no dependency on the rest).
* The subscriber writes `reclaw.log` in the platform's state folder (`~/.local/state/reclaw/logs` on Linux), plus the terminal when
  there is one. The file rotates by size (2 MiB, five files), so a retry loop cannot fill a disk and the last few runs are always there.
  Each event is written whole, with the level, the thread name and the module.
* Our own crates log at `info`, everything else at `warn`; Settings, Diagnostics offers Problems only / Normal / Detailed, and
  `RECLAW_LOG=reclaw_net=debug,warn` overrides it. The level changes while running.
* **Credentials never reach a file**, three ways: the `Secret` type prints as nothing, every pasted or loaded token is registered so its
  exact text is removed from any line whatever its shape, and token-shaped text and credential headers are removed by pattern. An event
  is cleaned as one piece, so a token split across two writes is still caught.
* A panic is logged with its thread, place and stack before the usual message.
* Every notice the user sees is also logged, from `Store::dispatch`, the one place all actions pass, with its details. The failures a
  person saw and the failures the log has are then the same set.
* `tests/repo_hygiene.rs` fails on `println!`/`eprintln!` in the program's crates and on a workspace crate that is not a log target.

## Rejected

* **`log` with `env_logger` or `flexi_logger`.** Simpler, but the toolkit speaks `tracing`, and `tracing` gives fields (`host=...
  attempt=2`) a person can search for instead of sentences. `tracing-subscriber` also bridges `log` records, so the HTTP stack's notes
  arrive anyway.
* **`tracing-appender`.** Rolls by time, which fits a daemon, not a program started a few times a week; its non-blocking writer drops
  the last lines in a crash, which is when they matter; and it adds dependencies for something a hundred lines do here.
* **journald / systemd.** Not on Windows or macOS, and a person cannot be asked to run `journalctl` with the right filter. A file can
  be attached to a message.
* **`env-filter`.** Pulls in a regex engine to read one directive string. `Targets` parses the same syntax without it.
* **An in-app log viewer.** Worth having later; it is not needed to find an error, and a button that opens the folder is enough now.
* **Logging only when asked (a debug flag).** The failure that matters has already happened by the time anyone asks.
