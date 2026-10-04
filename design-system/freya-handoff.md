# Freya handoff

How to get from this system into the Rust app. Written against the skills you supplied (freya 0.4.0, design-to-freya); anything marked unverified was not compiled here.

## Pipeline

1. Tokens: `tokens.json` is the source. `freya/theme.rs.txt` (save as theme.rs) is generated from it (a `Reclaw` struct of every token and two `Theme` constructors, `reclaw_midnight` and `reclaw_daylight`). Provide the struct with `use_provide_context` beside `use_init_theme`.
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

Hanken Grotesk and JetBrains Mono are open-licence families but are not bundled in this system. Download the TTFs, embed them with `include_bytes!` and register them in the app's font settings (unverified: confirm the 0.4.0 fonts API). Until then Freya falls back to the system sans.

## Open items

- Real icon set: confirm each Lucide name in the contract exists in your pinned `freya-icons` submodule.
- Catalog art: capsule 3:4 and hero 16:5 are the two crop ratios the design assumes.
- Gamepad focus order is specified as a ring only; the traversal order per screen is not designed yet.
