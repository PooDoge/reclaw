# Freya handoff

How to get from this system into the Rust app. Targets freya 0.5.0-rc.8 (newest on crates.io). The Rust implementation lives in `reclaw-ui/` in the repo; it compiles, passes clippy with warnings denied, and renders headless snapshots at each layout class. The skills you supplied describe 0.4; the API differences that bit are listed at the end.

## Pipeline

1. Tokens: `tokens.json` is the source. `freya/theme.rs.txt` (the repo's `reclaw-ui/src/tokens.rs`) is generated from it (a `Reclaw` struct of every token and two `Theme` constructors, `reclaw_midnight` and `reclaw_daylight`). Provide the struct with `use_provide_context` beside `use_init_theme`.
2. Contract: `reclaw.freya.json` lists every component with its Freya built-in (or `custom`), tokens, radius, states and responsive behavior. Treat it as the WHAT; the design-to-freya skill is the HOW.
3. Previews: `components/*/preview.html` are the visual reference. They are React stand-ins, not code to port.

## Component map

| design | Freya |
| --- | --- |
| Button | `Button` filled / outline / flat with `theme_colors` |
| Chip | `Chip` |
| StatusBadge | custom `rect()` + Lucide icon + `label()` |
| Switch | `Switch` |
| SearchField | `Input` (give it an explicit background) |
| GameCapsule | `Card` + `image()` |
| LibraryRow | `SideBarItem` |
| HeroHeader | custom `rect()` over `image()` |
| DownloadItem | `ProgressBar` in a custom row |
| Nav | `SideBarItem` / `FloatingTab` (rail, tabs), custom top bar |
| InstallDialog | `Popup` (+ `PopupTitle`, `PopupContent`, `PopupButtons`) |
| Library grid | `VirtualScrollView` |
| Responsive shell | `ResizableContainer` is not needed; switch on width state |

## Fonts

Hanken Grotesk and JetBrains Mono are open-licence families but are not bundled. Download the TTFs and register them with `LaunchConfig::with_font`. Until then Freya falls back to the system sans (the snapshots show this).

## Open items

- Real icon set: confirm each Lucide name in the contract exists in your pinned `freya-icons` submodule.
- Catalog art: capsule 3:4 and hero 16:5 are the two crop ratios the design assumes.
- Gamepad focus order is specified as a ring only; the traversal order per screen is not designed yet.

## Deck mode handover

The full machine-readable spec is `reclaw.freya.json` (`decisions`, `modes`, `input`, `focus`, `lifecycle`, deck `components`, `verification`, `notBuilt`). Rust lives in three workspace crates:

| crate | holds | tested by |
| --- | --- | --- |
| `reclaw-input` | `ActionMap`, `InputMapper` (repeat, stick hysteresis, input ownership), `ControllerKind` and glyph faces, `next_focus`, `detect_environment`, optional `backend` (gilrs) | 26 unit tests |
| `reclaw-runtime` | `Supervisor` (own process group, graceful stop, force on second press), `RunState`, `Outcome`, `InputProfile`, `SdlMapping` | 14 tests with real processes, including killing children and ignoring SIGTERM |
| `reclaw-ui::deck` | `DeckState` reducer (pure), `launch_verb`, deck components, `DeckApp` | 25 unit, 5 snapshot, 1 end-to-end lifecycle test |

### Deck component map

| contract | Rust | built from |
| --- | --- | --- |
| DeckTile | `deck::DeckTile` | custom `rect`, `FocusFrame` |
| Shelf | `deck::Shelf` | clipped `rect` + `offset_x` |
| ButtonGlyph, HintBar, SectionTabs | `deck::{ButtonGlyph, HintBar, SectionTabs}` | custom `rect`, Lucide icons for PlayStation shapes |
| SlidePanel, MainMenu, QuickAccess | `deck::{SlidePanel, MainMenu, QuickAccess}` | absolute `rect` on `Layer::Overlay`, `use_animation_transition` |
| LaunchButton, NowPlayingBanner | `deck::{LaunchButton, NowPlayingBanner}` | `ActionButton` in `FocusFrame` |
| Backdrop | `deck::Backdrop` | `ImageViewer` + `blur`, `LinearGradient` wash |
| DeckApp | `deck::DeckApp` | root `rect`, `on_sized`, `on_global_key_down`, a task draining `ActionFeed` |

### Wiring recipe

1. Host root: `use_init_reclaw`, own `State<Vec<GameEntry>>`, `State<Vec<Download>>`, `State<Option<ControllerInfo>>`.
2. `backend::spawn(map)` for the pad (feature `gamepad`); forward `InputMessage::Action` into `ActionFeed::sender()` and connection messages into the controller state.
3. `Supervisor::new(grace)`; on `SessionEvent` write `supervisor.state(app)` into the game's `run`.
4. Pass an `on_effect` handler: `Launch` and `Stop` go to the supervisor, `InputOwner` goes to the pad handle, the rest (`Resume`, `BringLauncherToFront`, `Install`...) are window and installer work for the host. `examples/deck.rs` is the reference.
5. Under Steam, `map.unbind(Button::Guide)` (see `detect_environment`).

### Deck API notes (0.5.0-rc.8)

- Animation lives in `freya::animation::*`, not the prelude.
- Absolute children are positioned from their **parent's origin**; give an overlay root `Position::new_absolute().top(0).left(0)` and its own size, or it lands after the previous sibling.
- Overlays need `.layer(Layer::Overlay)` or page content paints over them.
- `TestingRunner::press_key` and `poll` make keyboard-driven and animated components testable headlessly; `poll` is real time.
- `EventHandler` props never compare equal across renders, so components that receive one re-render with their parent.

## 0.4 to 0.5 differences found while building

- `Input` layout theme: `inner_margin` is now `padding`.
- `Tooltip::new()` takes no text; use `Tooltip::new_text(..)`.
- Cursor is declarative: `.cursor(CursorIcon::Pointer)` replaces `Cursor::set` in pointer handlers.
- `ProgressBar` has no `.theme(..)`; use the generated setters `.background(..)`, `.progress_background(..)`, `.height(..)`, and `.show_progress(false)`.
- `State<T>` is only callable (`state()`) for `Copy` T; use `.read().clone()` for `Option<String>` and the like.
- Absolute `Position` anchored with `bottom` does not behave as in CSS; place by `top` with a fixed height.
- Building needs `prettyplease` 0.3 (`cargo update -p prettyplease`) because bindgen resolves to syn 3, and the system GL libs (`libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev libwayland-dev`) to link.
