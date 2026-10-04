# reclaw-ui

Freya components for the Reclaw installer and launcher, generated from the design system contract
(`../design-system/reclaw.freya.json`). Targets **freya 0.5.0-rc.8** (the newest release on
crates.io; there is no final 0.5.0 yet), pinned exactly in `Cargo.toml`.

```
cargo run --example gallery          # RECLAW_THEME=daylight  RECLAW_DENSITY=touch
cargo test                           # writes target/snapshots/*.png (headless Skia renders)
```

| contract component | Rust | Freya built-in |
| --- | --- | --- |
| Button | `ActionButton` | `Button` (themed) |
| Chip | `FilterChip` | `Chip` (themed) |
| StatusBadge | `StatusBadge` | custom `rect` |
| Switch | `ToggleSwitch` | `Switch` (themed) |
| SearchField | `SearchField` | `Input` (themed) |
| GameCapsule | `GameCapsule` | custom `rect` (needs the accent hover border) |
| LibraryRow | `LibraryRow` | custom `rect` (`SideBarItem` active state is route-driven) |
| HeroHeader | `HeroHeader` | custom `rect` |
| DownloadItem | `DownloadItem` | `ProgressBar` in a custom row |
| Nav | `Nav` (`Top`, `Rail`, `Bottom`) | custom `rect` + `TooltipContainer` on the rail |
| InstallDialog | `InstallDialog` | `Popup` |

Layout class comes from the root width (`LayoutClass::from_width`, set in `ReclawApp` through
`on_sized`); density (`Density::{Pointer, Touch}`) is a separate switch.

`src/tokens.rs` is generated from `design-system/tokens.json`; do not hand-edit it.
Fonts (Hanken Grotesk, JetBrains Mono) are referenced by family name only and fall back to the
system font until you register the TTFs with `LaunchConfig::with_font`.
