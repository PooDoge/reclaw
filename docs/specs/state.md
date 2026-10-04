# Application state and persistence

- last-verified: 2026-10-04
- owner-paths: reclaw-ui/src/store/**, reclaw-ui/src/bootstrap.rs, reclaw-ui/src/settings/**, reclaw-config/src/**, reclaw-ui/tests/ui/shell_modes.rs

What state exists, where each kind lives, how it changes, and what survives a restart.

## Three kinds of state

| Kind | Lives in | Example |
|---|---|---|
| Shared by every window, survives a mode switch, or comes from outside the UI | the **store** (`freya-radio`) | the library, downloads, notifications, settings, what the display offers |
| One window's business | `use_state` in the component that owns it, or context from the frame (`Nav`, `DesktopUi`, `ShellCtx`) | the page, the search text, an open dialog, focus in Deck mode |
| Fetched, cached, can be fetched again | the media cache on disk (`docs/specs/media.md`) | artwork, README text |

The store is global (`Store::create_global`), so a second window can use it: it calls `install()` first and reads like any other.
Tests use `Store::use_scoped`.

## The store

One `AppState` (plain data, no Freya types) changed only by `AppAction`s. `reduce` is pure: it applies one action and returns the
`AppChannel`s it touched. A write notifies those channels and no others, so a download's progress does not redraw the settings
page. Components read with `use_games()`, `use_settings()` ... and never reach into the station. Threads that are not the UI's
(the supervisor, a downloader, the gamepad) cannot touch the store; they send actions through a `StoreFeed` and one task on the UI
thread dispatches them.

**Adding a piece of state:** a field on `AppState` (and a line in `to_prefs`/`from_prefs` if it must survive), a channel if readers
should not redraw for unrelated changes, an action and its arm in `reduce` with a test, and a `use_...()` hook.

## Settings as data

Reclaw's settings are a schema (`settings/builders.rs`: sections of groups of rows) shown by both interfaces. A value is saved
**by the row's key and, for a choice, by its position**; adding a row needs no migration, and an unknown key from a newer file is
ignored. Other code reads a setting through a named constant (`KEY_THEME`, `KEY_LIBRARY_SORT`, `KEY_DECK_DISPLAY` ...); renaming
one loses what people chose. Launch settings are a separate layered model (`docs/specs/game-settings.md`).

## What is saved

`reclaw-config` writes one TOML file (`<config>/settings.toml`): the settings (global and per game), launch defaults and per-game
overrides, favorites, and the window's size, place, maximized state and monitor. Everything else is rebuilt at start.

* **Tolerant to read**: every field has a default; a file that cannot be parsed is set aside as `settings.toml.bad` and the app
  starts with defaults and a warning; a file from a newer Reclaw (higher `version`) is never overwritten, the app runs with its
  settings unsaved.
* **Safe to write**: a temporary file renamed over the real one. Writes go through `PrefsWriter`, a background thread that waits
  400 ms for further changes and writes once; `Store::flush` writes now and waits (called when the window closes; the window's own
  Close button does it too, because the toolkit's close call skips the host's close hook).
* **Where**: the platform's config directory, or everything under `RECLAW_HOME`. If no folder can be found the app starts with
  settings that are not saved.

## Tests

`store/tests/{reduce,persistence}.rs`, `bootstrap.rs` (a change saved and read back, a damaged file, a newer file),
`reclaw-config` unit tests (atomic write, tolerant load, migration, the writer).
