# Reclaw architecture

Reclaw is a Steam-familiar installer and launcher for recompiled games, written in Rust with
[Freya](https://github.com/marc2332/freya) 0.5 (pinned to `=0.5.0-rc.8`; there is no final 0.5.0 yet).
It has two interfaces over one set of data: a **desktop** interface and **Deck mode**, a gamepad-first
interface in the style of Steam Big Picture and the Steam Deck UI.

The design lives in `design-system/` (tokens, the Freya handoff contract `reclaw.freya.json`, per-component
previews). The code in `reclaw-ui` implements it; `design-system/freya-handoff.md` says how the two map.

## Crates

| Crate | Job | Depends on Freya |
|---|---|---|
| `reclaw-input` | Pad buttons to `Action`s, SDL mapping, spatial focus (`next_focus`), environment detection, the gilrs reader (`gilrs-backend` feature) | no |
| `reclaw-runtime` | Launching apps as process groups, the `Supervisor` (Stop, force-kill, session events), controller `InputProfile` for the launched app | no |
| `reclaw-ui` | Everything you see: tokens, components, the surface system, Deck mode, the `Shell` that switches interfaces | yes |

`reclaw-input` and `reclaw-runtime` never import Freya, so their tests run without a window.

## Inside `reclaw-ui/src`

```
lib.rs            module map + prelude
tokens.rs         GENERATED from design-system/tokens.json (do not edit; see "Tokens")
theme.rs          ThemeKind, Reclaw (the tokens struct), use_init_reclaw / use_reclaw
metrics.rs        sizes, LayoutClass (Wide/Compact/Phone), Density (Pointer/Touch/Controller)
typography.rs     TypeStyle
icon.rs           IconName (Lucide)
model.rs          GameEntry, Download, AppStatus ... plain data
sample.rs         sample data for the gallery and tests
effect.rs         Effect: every command the UI asks of its host (shared by both interfaces)
host.rs           HostState: the states host and UI share (games, downloads, pad, keyboard inset, chosen file)
app_menu.rs       the per-app Options menu as data (shared by both interfaces)

components/       desktop building blocks, one component per file
screens/library/  the desktop Library: mod.rs + wide.rs / compact.rs / phone.rs + parts.rs, ctx.rs, filter.rs
desktop.rs        DesktopApp: measures the window, picks the layout class, shows the Library
app.rs            ReclawApp: desktop-only root with its own theme, for the gallery and snapshots

surface/          dialogs, full-screen pages, menus: the rules for which to use where (see below)
deck/             Deck mode
  state/          the reducer: DeckState, actions in, Effects out. NO Freya.
  settings/       settings and properties as data (schema, values, row geometry). NO Freya.
  launch.rs       what the Play button says for each run state. NO Freya.
  layout.rs       shelf scroll maths. NO Freya.
  widgets/        single Deck components (tile, hint bar, banner, panels ...)
  pages/          what fills the body between the tabs and the hint bar
  app/            DeckApp: wires input, state and rendering (see its mod.rs docs)
shell/            Shell: shows desktop or Deck, F10 / F9, dev overrides. model.rs is pure.
```

### The rule that keeps this testable

> **Decisions are pure data in, data out. Components only draw.**

`deck/state`, `deck/settings`, `surface/{presentation,menu,reveal}`, `shell/{model,overrides}`,
`app_menu`, `screens/library/filter` contain no Freya types and are unit-tested next to the code
(`cargo test -p reclaw-ui --lib`, milliseconds). A component receives what it should show and
reports what the user did through `EventHandler`s; it does not decide.

## Surfaces: dialog, page or menu?

`surface::presentation(kind, ctx)` is the single table (`SurfaceContext` = layout class, density, window height):

| Kind | Desktop (pointer, wide) | Phone, short touch screen, Deck mode |
|---|---|---|
| **Form** (inputs, install, settings) | Popup over a dimmed page | **Full-screen page** with a Back button, scrolling body, footer actions |
| **Confirm** (a sentence, two buttons) | Popup | Small centered card over a darkened screen |
| **Menu** (a list of choices, may cascade) | Anchored popover at the pointer | **Centered over a darkened screen**, focused row inverted, submenus open to the right |

"Short touch screen" = touch density and under 900px tall. Callers describe content and actions once;
`Dialog` picks the presentation. Do not branch on form factor inside a screen.

**Full-screen pages** (`FullScreenPage`): header with Back and title; body in a centered column (or
`wide`/`fixed` for two-pane); footer actions (moved into the header under 600px tall); hint bar in Deck.

**Keyboard avoidance.** The host writes the on-screen keyboard height to `HostState::keyboard_inset`.
While it is above zero the footer and hints hide, the header shrinks if the keyboard covers over 40% of
the window, the body grows a spacer, and `scroll_to_reveal` keeps the focused field inside
`visible_height(...)`. Use `F9` (or `RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD`) to see it on a desktop.

## Deck mode

* `DeckState` (reducer) turns `Action`s (from the pad or the keyboard fallback) into state changes and `Effect`s.
  Focus is a set of declared rectangles; `reclaw_input::next_focus` does spatial navigation.
* `DeckApp` owns no data: it reads `HostState` and reports `Effect`s. The host starts processes, opens
  windows and files. The `Supervisor` and `DeckApp` meet only through `HostState.games[..].run`.
* While an app runs it owns the pad; only Guide reaches the launcher (`Effect::InputOwner`).
* Text boxes: entering one from the pad or keyboard **clears it** (restoring the old text if left empty)
  because Freya's `Input` cannot move its caret from outside. A tap or click keeps the text.
* Deck buttons are not in Freya's Tab order (`ManagedFocus`), or Enter would press them twice.

## Switching interfaces

`Shell` is the root. It chooses desktop or Deck from `detect_environment` (`RECLAW_MODE`, SteamOS
variables), and the user can change it by hand: **F10**, the desktop's "Deck mode" button, Deck's main
menu "Switch to desktop", or Settings > Interface. `DevOverrides` (`RECLAW_LAYOUT`, `RECLAW_DENSITY`,
`RECLAW_THEME`, `RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD`) push any build into any form factor.

## Tokens

`design-system/tokens.json` is the source. `python3 gen_tokens.py` (in the design-system tooling) writes
`tokens.json`, `freya/theme.rs.txt` and `reclaw-ui/src/tokens.rs`; `tests/tokens_in_sync.rs` fails if the
Rust drifts from the JSON.

## Testing map

| Where | What |
|---|---|
| `src/**` `#[cfg(test)]` and `deck/state/tests/` | Pure logic. Fast. Add a test here first. |
| `tests/common/` | The harness: `Mount::deck()` / `Mount::desktop()` mount the real `Shell` headless; `Session` presses keys, sends pad actions, moves the keyboard, reads labels and their positions, writes PNGs to `reclaw-ui/target/snapshots/`. |
| `tests/deck_input.rs` | Keys through the real Deck component to effects and screen. |
| `tests/deck_surfaces.rs`, `deck_snapshots.rs` | Deck pages and menus at each form factor (PNGs). |
| `tests/desktop_surfaces.rs`, `desktop_snapshots.rs` | Dialogs and menus on the desktop at each form factor. |
| `tests/shell_modes.rs` | Entering and leaving Deck mode by hand. |
| `tests/deck_lifecycle.rs` | Play, Guide, Resume and Stop with a real child process (Unix). |
| `tests/tokens_in_sync.rs`, `tests/repo_hygiene.rs` | Drift and file-size rules. |

## Not built yet

An in-app on-screen keyboard (the OS provides one; here it is simulated), a gamepad file browser for
"choose your game file" (`Effect::ChooseFile` asks the host), a desktop Settings route (Properties is
Deck-only), the catalog and mods pages, and verification on real SteamOS and Windows hardware.
