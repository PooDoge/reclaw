# Working in this repository

Read `ARCHITECTURE.md` first; it is the map. This file is the short list of rules.

## Commands

```sh
cargo test --workspace                      # everything (a few minutes the first time: Skia)
cargo test -p reclaw-ui --lib               # the pure logic, instant
cargo test -p reclaw-ui --test ui deck_notices::   # one module of the UI binary (tests/ui/main.rs lists them)
cargo test -p reclaw-ui --test repo_hygiene        # file sizes, //! headers, docs rules
cargo clippy --workspace --all-targets      # must stay clean (CI uses -D warnings)
cargo fmt --all                             # rustfmt.toml sets the style; run before committing
QUIVER_CATALOG_DIR=<checkout> cargo test -p reclaw-catalog   # also check the real community catalog (skipped without it)
cargo run -p reclaw-ui --example gallery    # look at it (F10: Deck mode, F9: simulated keyboard)
cargo run -p reclaw-ui --example deck --features gamepad   # with a real pad and processes
scripts/x11-smoke.sh                        # a real window under Xvfb: drag, resize, maximize, close (not part of cargo test)
```

Linux needs `libudev-dev` and the GL/EGL dev packages (`libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev libwayland-dev`).
Snapshot PNGs land in `reclaw-ui/target/snapshots/`; look at them after changing any layout.
Bazzite (immutable) builds in a distrobox: `docs/BUILDING.md`, `scripts/bazzite-build.sh`.
Skia makes `target/` huge. If the disk fills, `cargo clean -p reclaw-ui -p reclaw-media -p reclaw-games -p reclaw-input -p reclaw-config -p reclaw-runtime -p reclaw-catalog`
keeps the compiled dependencies and drops only ours.

## Rules

1. **Files stay under 1000 lines, and most under 300.** `tests/repo_hygiene.rs` enforces the cap. Split by
   responsibility (one component, one reducer concern, one layout class per file), not by line count.
2. **Every `mod.rs` and `lib.rs` starts with a `//!` comment** naming the module's job and listing its files.
3. **Decisions are pure; components draw.** New behavior goes in a Freya-free module with unit tests
   (`deck/state`, `store/reduce.rs`, `activity`, `notices`, `nav/{route,transition}`, `window/{monitors,geometry,policy}`,
   `surface/*`, `shell/model.rs`), then a component shows it.
4. **Tests are not in the same file as a lot of code.** A few inline unit tests are fine; more than ~150 lines moves to a
   sibling `tests.rs`/`tests/` folder. UI integration tests are one binary (`tests/ui/main.rs` lists the modules) and share `tests/ui/common/`.
5. **Hooks are never conditional.** `use_state`, `use_reclaw()`, `use_hook` ... must run on every render in the same order.
   Read the tokens once at the top of a component and pass them down; do not call `use_reclaw()` in a helper that only some
   code paths reach (this panicked on resize and on drilling into settings).
6. **One host vocabulary.** Anything the UI asks of the outside world is an `Effect` (`effect.rs`). Do not call the OS from a component,
   and do not call winit or `Platform` from one either (`window::WindowCommand` is the window's share of the vocabulary).
7. **Form factor is decided in one place**: `surface::presentation`. Screens describe content; `Dialog`/`FullScreenPage`/`ModalMenu` present it.
8. **Tokens come from `design-system/tokens.json`.** Never edit `src/tokens.rs`; regenerate it.
9. **No `unwrap()` outside tests, no empty `catch`-style swallowing**; return or report errors.
10. **Say what you could not verify.** Real gamepad hardware, SteamOS environment variables, Wayland/GNOME and Windows are unverified;
    the window frame is verified on X11 only. `docs/BUILDING.md` has the table.
11. **Shared state has one home.** Anything two windows, both interfaces, or a background thread must agree on lives in `AppState` and
    changes through an `AppAction` (`store/`); one window's own state stays in `use_state`. Do not mirror store data into local state.
12. **Only `reclaw-media` fetches.** Never turn on the toolkit's `remote-asset` or `html` features and never fetch from a component;
    ask `use_remote_file`. Remote text is untrusted: markdown goes through `reclaw_media::readme` first.
13. **Docs move with behavior.** A change in behavior updates `docs/specs/<area>.md` in the same commit (and its `last-verified` date).
    A decision that has a rejected alternative gets an ADR in `docs/adr/`. `tests/repo_hygiene.rs` checks the shape of both.

## Adding things

* **A desktop component**: `components/<name>.rs`, export in `components/mod.rs`, a preview in `design-system/components/<Name>/`,
  an entry in `reclaw.freya.json`.
* **A Deck page**: `deck/pages/<name>.rs`; focus targets in `deck/state/ids.rs` and `nodes.rs`; its screen in `deck/app/screens.rs`;
  a reducer test in `deck/state/tests/`.
* **A settings row**: a `Row` in `settings/builders.rs` (a key, a label, a kind). Both interfaces render it from the schema, focus
  follows, and a choice is saved by position, so append options, never reorder them. Keep descriptions short; rows are one line high.
  A setting the rest of the app reacts to: read it with `use_settings()` and add its key as a constant beside the others.
* **A page**: a variant in `nav/route.rs` (with its metadata in `meta.rs`), a component in `pages/routes.rs`, then the desktop version in
  `desktop/pages/` and/or the Deck one in `deck/pages/`; a title in `nav/titles.rs` if it appears in Recent.
* **A piece of shared state**: a field in `AppState`, an `AppAction`, its arm in `reduce` and a test, an `AppChannel` if readers should be
  told separately, a `use_*` hook. If it must survive a restart, add it to `to_prefs`/`from_prefs` and a default in `reclaw-config`
  (new fields need no migration; renamed or removed ones do).
* **A background activity**: the host sends `ActivityEvent`s; the board and the indicators decide what is shown. Do not set a card's
  status from a component.
* **A window command**: a `WindowCommand` variant, handled in `window/platform.rs`; the `Shell` runs it, a component only produces the effect.
* **A picture or document from the web**: `use_remote_file(url)` in a component keyed by the URL; the policy lives in `reclaw-media`.
* **An Effect**: variant in `effect.rs`, produced by the reducer or a component handler, handled in the host (`examples/deck.rs`
  shows one). The `Shell` handles mode effects itself.
* **A dialog**: build a `Dialog` with a `SurfaceKind`; do not use Freya's `Popup` directly.
* **A menu**: `MenuState` data + `ModalMenu`; see `app_menu.rs`.

## Freya 0.5 notes that cost time

Absolute `Position` is relative to the parent's origin and `bottom()` does not anchor like CSS (use `top`). `EventHandler`s are never
equal across renders. Overlays need an absolute root with its own size and `Layer::Overlay`. Freya's wrapper moves focus on Tab and
the vertical arrows. `Input` starts its caret at 0. `State<Option<String>>` cannot be called like a function (not `Copy`). Sibling order did not decide paint order between an overlay's scrim and its panel (the scrim covered the panel's own background): put the scrim on `Layer::Relative(-1)`. `Platform::root_size` is the window in physical pixels while `Position` and `Size::px` are layout units (physical / display scale / UI scale): never place anything from `root_size`; use a size the shell measured (`on_sized`). The toolkit's resize bands do exactly that, which is why `window::ResizeBands` exists.

## Freya source of truth

The reference for the Freya API is the source that compiles: `~/.cargo/registry/src/*/freya-*-0.5.0-rc.8/` (crates.io release
`0.5.0-rc.8`, published from tag `v0.5.0-rc.8` = commit `af55a77c3cb818c74d469d7da97f834f28cc1f0a`; each crate's `.cargo_vcs_info.json`
says so). Check the version in the directory name: the registry may also hold 0.4.x. A Freya git checkout is a reference only if it is
that tag; a fork or `main` can be far from it (a fork snapshot seen in development was an `rc.1`-era tree and differed from rc.8 in 48
files of `freya-core`). `tests/repo_hygiene.rs` fails if any `freya*` crate in `Cargo.lock` is not `0.5.0-rc.8` from crates.io, if a
Freya dependency is not pinned with `=`, or if a `[patch]` appears. To move to a new release, re-check every claim about Freya in
`docs/adr` and `docs/specs` against the new source, update their `freya:` lines, then change the constant in that test.
