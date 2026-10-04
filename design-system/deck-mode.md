# Deck mode

The controller-first shell for handhelds and TVs, in the language of Steam's Big Picture and the Steam Deck UI: big art, one focused thing at a time, slide-in overlays, button prompts. It is a **mode**, not a theme and not a second app: the data, installer and launcher are shared, and only the shell changes.

## What the reference said, and what we kept

The brief arrived as a summary of Valve's unified interface. We kept the ideas that hold up and did not copy its numbers.

| idea | decision here |
| --- | --- |
| Big art | Full-bleed `Backdrop` follows the focused game: its art, blurred, under `deck-scrim`. Text never sits on the art itself. |
| Focus grows and glows | Focused tile scales 1.06, gets a 3px solid accent ring and a `focus-glow` halo. The ring carries the meaning; the glow is decoration. |
| Shelves | Horizontal rows (`Continue`, `All apps`); the focused tile stays centered; the page snaps to the focused shelf. |
| Slide-in overlays | Main menu from the left, Quick Access from the right, both over the page; focus trapped; Back closes. |
| Glyph prompts | A bottom `HintBar` whose glyphs follow the controller in use, or keycaps if the keyboard was used last. |
| Dense desktop vs big console | `Density::Controller`: rows 64, targets 56, body 20, safe zone 48x32. These sizes are ours, chosen for reading distance; they are not Valve's. |
| Theme | Deck follows the theme (ADR 0007): `deck-bg`, `deck-scrim`, `focus-glow`, `deck-dim` and `deck-dim-ink` each have a value per theme. |

## Principles

- **One tree.** Shared components read the density; deck-only components exist only where the pattern is new (tiles, shelves, hints, panels, the Play/Stop pair).
- **The pad has one owner.** Launcher or App. While an app is in front the launcher ignores the pad except Guide.
- **Don't clone Steam.** Under Steam's Gaming Mode the Guide button and overlay are Steam's; Reclaw removes its Guide binding and keeps its own navigation, tiles and lifecycle.
- **Focus is declared.** Every screen declares rectangles from layout constants; a pure function moves between them. Hover never moves focus; click is focus plus confirm.
- **State is in words.** Every state has a word as well as a color or icon; failures say why and offer Retry and the log.

## Screens

- **Home:** section tabs with bumper glyphs, Now Playing banner while an app is active, two shelves, hint bar.
- **Game:** project eyebrow, `deck-title`, badge + version + source, the launch verb (Resume + Stop while running), Open folder, Manage.
- **Downloads:** queue with Cancel as the focus target. **Catalog and Mods:** empty states.
- **Install:** a full-screen page (Back, location, your game file, two switches, Cancel and Install). **Settings and Properties:** full-screen pages built from a schema, two panes or a drill-down. See the Surfaces section.
- **Overlays:** Main menu (Library, Catalog, Downloads, Mods, Settings, Switch to desktop mode); Quick Access (Now playing with Resume + Stop, controller name and battery, downloads); the **Options menu** (Menu button or `o`), centered over a darkened screen and cascading; a **confirmation** for Uninstall.
- **Hint bar:** the Menu and Quick access hints sit at the left edge, Options, Select and Back at the right, as in Big Picture.

## Not designed or built yet

An in-app on-screen keyboard; a gamepad file browser (the native file picker cannot be driven by a pad, and the install flow needs one); the Catalog and Mods pages; a virtual keyboard/mouse device for apps that ignore SDL; window management for bringing the launcher and app forward; Windows Guide-button capture. Deck mode narrower than about 520px is usable but not designed.
