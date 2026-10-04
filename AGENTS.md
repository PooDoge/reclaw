# Working in this repository

Read `ARCHITECTURE.md` first; it is the map. This file is the short list of rules.

## Commands

```sh
cargo test --workspace                      # everything (a few minutes the first time: Skia)
cargo test -p reclaw-ui --lib               # the pure logic, instant
cargo test -p reclaw-ui --test deck_input   # one integration file
cargo clippy --workspace --all-targets      # must stay clean
cargo fmt --all                             # rustfmt.toml sets the style; run before committing
cargo run -p reclaw-ui --example gallery    # look at it (F10: Deck mode, F9: simulated keyboard)
cargo run -p reclaw-ui --example deck --features gamepad   # with a real pad and processes
```

Linux needs `libudev-dev` and the GL/EGL dev packages (`libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev libwayland-dev`).
Snapshot PNGs land in `reclaw-ui/target/snapshots/`; look at them after changing any layout.

## Rules

1. **Files stay under 1000 lines, and most under 300.** `tests/repo_hygiene.rs` enforces the cap. Split by
   responsibility (one component, one reducer concern, one layout class per file), not by line count.
2. **Every `mod.rs` and `lib.rs` starts with a `//!` comment** naming the module's job and listing its files.
3. **Decisions are pure; components draw.** New behavior goes in a Freya-free module with unit tests
   (`deck/state`, `surface/*`, `shell/model.rs`), then a component shows it.
4. **Tests are not in the same file as a lot of code.** A few inline unit tests are fine; more than ~150 lines moves to a
   sibling `tests.rs`/`tests/` folder. Integration tests share `tests/common/`.
5. **Hooks are never conditional.** `use_state`, `use_reclaw()`, `use_hook` ... must run on every render in the same order.
   Read the tokens once at the top of a component and pass them down; do not call `use_reclaw()` in a helper that only some
   code paths reach (this panicked on resize and on drilling into settings).
6. **One host vocabulary.** Anything the UI asks of the outside world is an `Effect` (`effect.rs`). Do not call the OS from a component.
7. **Form factor is decided in one place**: `surface::presentation`. Screens describe content; `Dialog`/`FullScreenPage`/`ModalMenu` present it.
8. **Tokens come from `design-system/tokens.json`.** Never edit `src/tokens.rs`; regenerate it.
9. **No `unwrap()` outside tests, no empty `catch`-style swallowing**; return or report errors.
10. **Say what you could not verify.** Real gamepad hardware, SteamOS environment variables and Windows are unverified.

## Adding things

* **A desktop component**: `components/<name>.rs`, export in `components/mod.rs`, a preview in `design-system/components/<Name>/`,
  an entry in `reclaw.freya.json`.
* **A Deck page**: `deck/pages/<name>.rs`; focus targets in `deck/state/ids.rs` and `nodes.rs`; its screen in `deck/app/screens.rs`;
  a reducer test in `deck/state/tests/`.
* **A settings row**: a `Row` in `deck/settings/builders.rs` (a key, a label, a kind). Layout and focus follow from the schema.
  Keep descriptions short; rows are one line high.
* **An Effect**: variant in `effect.rs`, produced by the reducer or a component handler, handled in the host (`examples/deck.rs`
  shows one). The `Shell` handles mode effects itself.
* **A dialog**: build a `Dialog` with a `SurfaceKind`; do not use Freya's `Popup` directly.
* **A menu**: `MenuState` data + `ModalMenu`; see `app_menu.rs`.

## Freya 0.5 notes that cost time

Absolute `Position` is relative to the parent's origin and `bottom()` does not anchor like CSS (use `top`). `EventHandler`s are never
equal across renders. Overlays need an absolute root with its own size and `Layer::Overlay`. Freya's wrapper moves focus on Tab and
the vertical arrows. `Input` starts its caret at 0. `State<Option<String>>` cannot be called like a function (not `Copy`).
