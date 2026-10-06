# 0019 A failed job shows its own share of the log

- status: accepted
- date: 2026-10-06
- spec: ../specs/notices-and-holds.md

## Context

A failed install said "Failed" and one line. Hovering gave nothing, and the only way to learn more was to find `reclaw.log` and read a
file shared by every thread, where the lines that explain one failure (a retry, a redirect, the file the policy refused) sit among the
catalog refresh, artwork fetches and other jobs. ADR 0013 put an in-app log viewer off as "not needed to find an error"; a person who
sees "Failed" on a game and nothing behind it shows that it is needed for this one case.

## Decision

* `reclaw-log` gains `record()`: a `Recording` with a span. A `tracing` layer installed with the logger copies every event inside that
  span (any crate, the same thread) into the recording, after the same level filter and the same redaction as the file, capped at the
  last 400 lines. The span is created at `error` so a strict level never disables it (a disabled span is never seen by the layer).
* The install worker runs inside a recording, logs "install started" and "release chosen" with the facts a reader needs, and on failure
  sends the lines as `ActivityEvent::Log` just before `ActivityEvent::Failed`.
* The UI keeps them on the `Activity`. Every desktop "Failed" indicator gets a tooltip with the reason cut to 80 characters, and a press
  opens a dialog with the whole reason, the details and the lines (warnings and errors in colour), plus "Open log folder" for the rest.
* A Failed state that outlived its job (after a restart) says so and opens the log folder.

## Rejected

* **Reading the job's lines back out of `reclaw.log`** (tail and filter by app). The file rotates, other threads interleave, and a
  filter on text finds lines that merely mention the app. The span is exact and costs nothing when no job runs.
* **A channel the job writes its own steps to.** The lines that explain a failure mostly come from code that knows nothing about jobs
  (the HTTP client's retries, the unpacker's refusals); a span reaches them without changing them.
* **A new field on `ActivityEvent::Failed`.** Every place that builds a `Failed` (mod downloads are being added beside this) would
  change; a separate `Log` event is additive and can also be sent for jobs that did not fail later.
* **Capturing at `debug` regardless of the Settings level.** It would show more than the file holds, and turning every debug call site
  on for all crates while a job runs has a cost. The recording follows the file; Detailed in Settings gives both more.
* **Keeping the recording on disk per job so it survives a restart.** The same lines are in `reclaw.log`, which the earlier-run case
  opens; a second store of log text is a second place for a credential to leak.
