# Reclaw architecture

Reclaw is a Steam-familiar installer and launcher for recompiled games, written in Rust with
[Freya](https://github.com/marc2332/freya) 0.5 (pinned to `=0.5.0-rc.8`; there is no final 0.5.0 yet).
It has two interfaces over one set of data: a **desktop** interface and **Deck mode**, a gamepad-first
interface in the style of Steam Big Picture and the Steam Deck UI. The first target to test on is Bazzite
(GNOME, Wayland); see `docs/BUILDING.md`.

The design lives in `design-system/` (tokens, the Freya handoff contract `reclaw.freya.json`, per-component
previews). The code in `reclaw-ui` implements it; `design-system/freya-handoff.md` says how the two map.
What each area does today is in `docs/specs/`; why it is that way is in `docs/adr/`.

## Crates

| Crate | Job | Freya |
|---|---|---|
| `reclaw-input` | Pad buttons to `Action`s, SDL mapping, spatial focus, press-and-hold (`HoldTracker`), environment detection, the gilrs reader (`gilrs-backend`) | no |
| `reclaw-runtime` | Launching apps as process groups, the `Supervisor` (Stop, force-kill, session events), the clean environment a game starts with, Wine / Proton / custom runners for Windows programs on Linux, controller `InputProfile` (spec: launch) | no |
| `reclaw-games` | What a project is (catalog metadata, art, media, releases), the systems games came from (`platform`), launch-setting capabilities and how a launch plan is built (`settings`) | no |
| `reclaw-catalog` | The catalog and the library in Quiver's format: apps, lists, the community index, the platform index, and the optional `reclaw` block (spec: catalog-format); quiverlauncher.com's types, linking, entries made from its apps and library follow (`site/`, spec: community) | no |
| `reclaw-log` | The log file, its redaction of credentials, the panic hook, the `Secret` type, one job's own lines (`record`); no dependency on the rest (spec: logging) | no |
| `reclaw-net` | The one HTTP client: honest user agent, retries, rate-limit and bot-check awareness, disk cache, resumable hashed downloads, access tokens that can change and be checked, `probe` (specs: network, credentials) | no |
| `reclaw-install` | Which release and which file fit this machine (Quiver's rules, ported), fetching releases from GitHub and GitLab, unpacking zip / tar.gz / tar.xz / 7z safely, the install itself (stage, lay over the folder, write the version last), uninstall with its refusals (spec: install) | no |
| `reclaw-mods` | Mods from Thunderstore and GameBanana: listing, the install (dependencies first, staged, never over another mod's files), removal, and Quiver's `.quiver-mods.json` record (spec: mods) | no |
| `reclaw-sync` | Loading the catalog (quiverlauncher.com's listing first, then the index, lists and platform metadata, ADR 0025) with offline fallback, and the library file with backups and a lock; the quiverlauncher.com client (`site.rs`) | no |
| `reclaw-config` | What is remembered between runs: one TOML file, tolerant load, atomic debounced save; the access tokens in their own private file | no |
| `reclaw-media` | Everything fetched from the internet to show: the address policy, the on-disk cache, the worker hub, README splitting | no |
| `reclaw` (`reclaw-app/`) | The program: the host that loads the catalog, keeps the library and the tokens, runs installs, updates, uninstalls, launches and mod jobs, writes the diagnostics report, updates from source, and answers the screens' requests; `main` starts the log and the window | yes |
| `reclaw-ui` | Everything you see: tokens, components, the surface system, both interfaces, the store, the router, the window frame, the `Shell` | yes |

Twelve crates never import Freya, so their tests run without a window or a GPU. `reclaw-ui` depends on the others, never the reverse.

## Inside `reclaw-ui/src`

```
lib.rs  prelude          tokens.rs GENERATED from design-system/tokens.json (never edit)
theme.rs metrics.rs typography.rs icon.rs   tokens struct, layout classes + densities, text styles, Lucide icons
model.rs catalog_data.rs plain data the UI shows (GameEntry, AppStatus ...), and the pure mapping from the catalog and the library to it
fixtures.rs              invented games for tests only (compiled for tests and the `fixtures` feature; the program has none)
effect.rs                Effect: every command the UI asks of its host (both interfaces)
app_menu.rs systems.rs   per-app Options menu as data; system badges, filter and sort by system
catalog.rs launch.rs     one game's view-model for its page; what Play says per run state
mod_games.rs mod_shelves.rs  the Mods tab's game choices; its shelves, search order and pages              (spec: mods)
search/                  the desktop search: which tab, open/close/send and the kept draft, matching, settings filter (spec: search)

store/                   shared state: AppState, AppAction, reduce, channels, Store handle, hooks  (spec: state)
activity/                installs, updates, mod downloads: the board, indicators, time left         (spec: state)
community/               what quiverlauncher.com says about a game: linking, wording            (spec: community)
catalog_browse.rs        the Catalog's sort, filters, hidden library apps and card notes (Quiver's Browse) (spec: community)
notices/                 messages about background events, and which button is held for what       (spec: notices-and-holds)
settings/                settings as data (schema, values, persistence, geometry, launch rows)
nav/                     Route enum, Nav handle, transitions, recents, layers, input               (spec: routing)
pages/                   one component per route; picks the desktop or Deck version
window/                  frame, monitors, geometry, policy, title bar, driver                      (spec: window)
media/  readme/          the Freya side of reclaw-media: use_remote_file, README on the markdown viewer (spec: media)
bootstrap.rs             opening the store, config and media hub for a host

components/              desktop building blocks, one component per file
surface/                 dialogs, full-screen pages, menus: which to use where
desktop/                 the desktop interface: frame, pages/*, dialogs/*
deck/                    Deck mode: state/ (reducer) settings/ widgets/ pages/ app/
shell/                   Shell: shows desktop or Deck, F9 / F10, dev overrides, services (the host's window + media)
```

### The rule that keeps this testable

> **Decisions are pure data in, data out. Components only draw.**

`store`, `activity`, `notices`, `settings`, `nav::{route,transition,recents}`, `window::{monitors,geometry,policy}`,
`deck/state`, `surface::{presentation,menu,reveal}`, `shell/{model,overrides}`, `systems`, `app_menu` and the
`reclaw-*` crates contain no Freya types and are unit-tested beside the code (`cargo test -p reclaw-ui --lib`, instant).
A component receives what it should show and reports what the user did through `EventHandler`s; it does not decide.

## The three kinds of state

Decided in `store/mod.rs` and ADR 0002; the short version:

| Kind | Where | Example |
|---|---|---|
| Shared across windows and modes, or arriving from outside the UI | `AppState` in the radio station, changed by `AppAction`s | library, activity, notices, settings, pad, display |
| One window's, thrown away with it | `use_state` / context in that window | search text, selected row, dialog open |
| Fetched, cached, can be stale | `reclaw-media` cache, read with `use_remote_file` | artwork, screenshots, README |

Components read with hooks (`use_games()`, `use_settings()` ...) that subscribe to one **channel**, so a progress tick redraws only what
shows progress. Other threads (a download, the pad, the supervisor) push actions through `StoreFeed`. What survives a restart is
`Preferences` in `reclaw-config`; `AppState::to_prefs/from_prefs` is the only bridge.

## How a change travels

```
pad / keyboard / mouse -> Action -> DeckState (pure) -> Effect --\
component handler ---------------------------------> Effect -----+--> Shell: handles Window, Mode, Navigate, Back
                                                                 \-> host: handles the rest (launch, install, open a file)
host / threads -> StoreFeed -> AppAction -> reduce -> AppState -> channel -> hooks -> redraw
```

Anything the UI asks of the outside world is an `Effect`. Components never call the OS, and never call the windowing library.

## Navigation

`nav::Route` is the only list of pages and `freya-router` is the truth about which one shows. Both interfaces use it;
`pages::*` picks the desktop or Deck component for the interface. `Nav` is the one way code changes page (`open`, `back`, `forward`,
`up`). Dialogs and menus register a layer, so Back closes them before leaving the page. `RouteStage` plays the transition (style and
intensity per interface and per page, pure maths in `nav::transition`). Recents are kept beside the router.
Deck's reducer keeps its own focus state and asks the router to follow; when the router moves for another reason Deck follows it.

## Surfaces: dialog, page or menu?

`surface::presentation(kind, ctx)` is the single table (`SurfaceContext` = layout class, density, window height):

| Kind | Desktop (pointer, wide) | Phone, short touch screen, Deck mode |
|---|---|---|
| **Form** (inputs, install, settings) | Popup over a dimmed page | **Full-screen page** with a Back button, scrolling body, footer actions |
| **Confirm** (a sentence, two buttons) | Popup | Small centered card over a darkened screen |
| **Menu** (a list of choices, may cascade) | Anchored popover at the pointer | **Centered over a darkened screen**, focused row inverted, submenus open to the right |

"Short touch screen" = touch density and under 900px tall. Callers describe content and actions once; `Dialog` picks the
presentation. Do not branch on form factor inside a screen. Keyboard avoidance: the host writes the on-screen keyboard height to the
store; while it is above zero the footer and hints hide and `scroll_to_reveal` keeps the focused field visible. `F9` simulates it.

## Deck mode

* `DeckState` (reducer) turns `Action`s (pad or keyboard fallback) into state changes and `Effect`s. Focus is a set of declared
  rectangles; `reclaw_input::next_focus` does spatial navigation.
* `DeckApp` owns no data: it reads the store and reports `Effect`s. It renders one `Frame` per render and the routed pages draw from it.
* While an app runs it owns the pad; only Guide reaches the launcher (`Effect::InputOwner`).
* Cards show an activity indicator (downloading / installing / update ready / done) and a progress bar; notices appear as a toast, and
  while one is up X (details) and Y (dismiss all) are **holds** with a ring in the glyph (spec: notices-and-holds).
* Text boxes: entering one from the pad or keyboard **clears it** (restoring the old text if left empty) because Freya's `Input` cannot
  move its caret from outside. A tap or click keeps the text. Deck buttons are not in Freya's Tab order (`ManagedFocus`).

## The window

The window has no native decoration and is transparent; `BorderlessPlugin` supplies resize bands and `window::titlebar` the drag area and
buttons. `RECLAW_WINDOW_FRAME=native` turns it off. Monitors come from winit once, become a `DisplayEnvironment` (the same type the launch
settings use), and a saved size and position are restored only if they still land on a connected monitor (spec: window).

## Network

Components never fetch (the same rule as every other call to the outside world: it is an `Effect` or a hook backed by a worker). Behind
them, code that talks to the internet shares one HTTP client from `reclaw-net` (ADR 0010, 0011): one user agent, proxy and certificate
settings, retries, conditional requests and resumable downloads. Pictures and READMEs written by strangers go through `reclaw-media`,
which adds an address policy (https, standard port, nothing on the local network, rechecked after redirects), size and time caps, kind
detection from the bytes and an on-disk cache. Components ask `use_remote_file(url)` and get a path in the cache or a placeholder.
READMEs are cut into blocks before the stock markdown viewer sees them (ADR 0004). The toolkit's own fetching is switched off (ADR 0003).

## Logging

`main` starts the log before anything else (`reclaw-log`, ADR 0013) and every crate writes with `tracing`; nothing prints. The file is
`reclaw.log` in the state folder, rotated by size, with credentials removed on the way in (the `Secret` type, registered exact values, and
patterns). Every notice the user is shown is also logged, from `Store::dispatch`. Settings, Diagnostics opens the folder, changes the
detail, and writes a report (versions, system, what each service answers, the end of the log). `docs/troubleshooting.md` lists the
lines and what to do. A job's own lines are also recorded beside it (`reclaw_log::record`): pressing a "Failed" label shows them (ADR 0019).
Spec: logging.

## Access tokens

Held in two places only: `secrets.toml` (private, beside the settings) and the network layer, which sends a token to its own service's
API host, checks it when it is pasted, and stops sending one the service refuses. The screens know only a `TokenStatus`. Settings, Network
is the one place to change them, on both interfaces. Spec: credentials (ADR 0014).

## Updating from source

For a copy built from a checkout: `scripts/update.sh` (a terminal) or Settings, About, "Update from source" (the same script, its output
going to the log) fast-forwards the branch, rebuilds and says to restart; it never resets or discards anything. Spec: updates (ADR 0015).

## Switching interfaces

`Shell` is the root. It chooses desktop or Deck from `detect_environment` (`RECLAW_MODE`, SteamOS variables), and the user can change it:
**F10**, the desktop's "Deck mode" button, Deck's main menu, or Settings. `DevOverrides` (`RECLAW_LAYOUT`, `RECLAW_DENSITY`, `RECLAW_THEME`,
`RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD`) push any build into any form factor. `Services` is what the host hands the shell (the window host
and the media hub); the shell's own effects are handled inside it.

## Tokens

`design-system/tokens.json` is the source. `python3 gen_tokens.py` (in `design-system/tools`) writes `tokens.json`, `freya/theme.rs.txt` and
`reclaw-ui/src/tokens.rs`; `tests/ui/tokens_in_sync.rs` fails if the Rust drifts from the JSON.

## Testing map

| Where | What |
|---|---|
| `src/**` `#[cfg(test)]`, `*/tests/` | Pure logic. Fast. Add a test here first. |
| `reclaw-*/tests/` | Crate-level tests: config files in a temp dir, a fake network for media, the real-network round trip (`--ignored`). |
| `tests/ui/` (one binary, `main.rs` lists the modules) | The real `Shell` mounted headless: `Mount::deck()` / `Mount::desktop()`, `Session` presses keys, sends pad actions, reads labels and positions, writes PNGs to `reclaw-ui/target/snapshots/`. |
| `deck_input`, `deck_routes`, `deck_notices`, `deck_surfaces`, `deck_snapshots`, `deck_lifecycle` | Deck: keys to effects and screen, routes, toasts and holds, pages at each form factor, a real child process (Unix). |
| `credentials` | The Network and Diagnostics settings on both interfaces: a pasted token leaves the page once, as a secret. |
| `reclaw-app/tests/update_script.rs`, `reclaw-net/tests/{credentials,logging}.rs`, `reclaw-log/tests/logging.rs` | The update script against real git repositories; tokens against a local server; the log file as a program writes it. `reclaw-net/tests/live.rs` (`--ignored`) asks the real GitHub. |
| `desktop_pages`, `desktop_settings`, `desktop_surfaces`, `desktop_snapshots` | Desktop pages, settings, dialogs and menus at each form factor. |
| `nav_stage`, `recents`, `shell_modes`, `window_chrome`, `systems`, `media` | Transitions, recents, mode switching, window commands, system badges and filter, artwork and README. |
| `tokens_in_sync.rs`, `tests/repo_hygiene.rs` | Token drift, file-size cap, `//!` headers, docs rules. |
| `scripts/x11-smoke.sh` | A real window under Xvfb + openbox: size, drag, resize, maximize, close. Not run by `cargo test`. |

## Not built yet

* Deck's Catalog and Mods pages, and the README on Deck.
* Drawing mermaid diagrams (they show as source), a cache size and "clear cache" in Settings.
* Mods: checked against a fake site only (the real Thunderstore and GameBanana were out of reach); no enable / disable, no chooser for a
  GameBanana mod with several files, no Deck page; the search covers the mods already listed, not the sites.
* Background update passes and auto-update (an explicit Check for updates exists), a chooser when several builds fit, choosing
  which program to start when there are several, a Windows-runner picker (the library's `linuxRunner` fields are honoured), a
  desktop or Steam shortcut, Flatpak bundles. `docs/quiver-parity.md` has the plan.
* The comparison that decides what in a catalog is new or changed against the library (Add / Merge / Replace).
* An in-app on-screen keyboard (the OS provides one; here it is simulated).
* Verification on real gamepad hardware, Wayland/GNOME, SteamOS and Windows. `docs/BUILDING.md` lists what was run and where.
