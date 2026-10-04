# 0002 Shared state in one freya-radio store, saved as a small TOML file

- status: accepted
- date: 2026-10-04
- spec: ../specs/state.md

## Context

Freya offers local state (`use_state`), context, `freya-radio` (global state shared by components and windows, with channels), and
`freya-query` (cached async data). The app may have several windows; it needs downloads, installs, updates and mod downloads with
progress; and it must remember the user's settings, theme, favorites and window across restarts, with new settings easy to add.

## Decision

* Three kinds of state, each in one place: **shared** state in one `freya-radio` station (`AppState`, changed only by `AppAction`s
  through a pure `reduce`); **one window's** state in `use_state` or frame context; **fetchable** data in the media cache, never the store.
* The station is global, so another window just calls `install()`. Readers subscribe to **channels** (games, activity, one game's
  activity, notices, settings ...), so a progress tick redraws only what shows progress.
* Other threads send actions through a feed drained by one UI task. `AppState` holds no handles.
* The saved part is a TOML file (`reclaw-config`): settings by key (choices by position), launch settings, favorites, window
  prefs. Every field has a default, so adding one needs no migration; an unreadable file is set aside; a newer file is never
  overwritten; writes are atomic and debounced, and flushed on close.
* Settings are **data**: a schema both interfaces render. Adding a row adds it everywhere.

## Rejected

* **Everything in context or `use_state` per window.** A second window and a mode switch would each need their own copy of the
  library and the downloads, kept in step by hand.
* **`freya-query` for the app's own state.** It fits fetched data with staleness; it has no notion of "the user changed a setting"
  or of a download's progress arriving from a thread.
* **One big signal with no channels.** Every progress tick would redraw the settings page.
* **SQLite or JSON for the settings.** The saved data is small, human-editable TOML is a feature, and a database adds a migration
  system for no gain. The library itself is not saved here: it will come from the install backend.
* **Saving the whole `AppState`.** It would save things that must be rebuilt (the library, downloads, the pad) and freeze their
  shape in the file.

## Consequences

Components read through hooks and never touch the station. Anything that must survive a restart needs a line in `to_prefs` and
`from_prefs`. Choice rows are saved by position, so reordering a row's options changes what people chose: append instead.
