Reclaw is the installer and launcher for recompiled games: it finds a recompilation project's release on GitHub or GitLab, installs it, checks it, and starts it. Its design language is deliberately Steam-familiar: a cool slate-navy shell, a left library list, portrait capsules, one green verb per screen. The design is for Freya (Rust, native), so every rule below maps to a token and a Freya built-in.

## Content fundamentals

- Say what the thing is, then what you can do. Buttons are verbs in sentence case: "Install", "Play", "Update", "Open folder". Never "Get started" or "Let's go".
- Address the user as "you/your" only where ownership matters: "Needs your game file". The launcher never says "we".
- Recompilation projects ship no copyrighted assets. Always say the user supplies their own file, and never offer to fetch it. Copy: "Choose the file you own".
- Errors name the cause and the next step: "Release asset not found. Check the repository or choose another version." Never "Something went wrong".
- Versions, tags, hashes, paths and asset names are set in `mono`. Game and project names are set in `body` or `title-*`. No emoji anywhere.

## Visual foundations

- **Surface ramp.** Five dark steps, darkest to lightest: `bg-deep` (title and status strips), `bg-nav` (navigation), `bg-base` (page), `bg-panel` (cards, capsules, dialogs), `bg-raised` (inputs, hover, selected). Cards always sit on a darker surface than themselves. `daylight` inverts the ramp; same names, same roles.
- **Text.** `ink` for primary, `ink-muted` for secondary and inactive nav, `ink-subtle` for placeholders and metadata. All three hold 4.5:1 on `bg-base`, `bg-panel` and `bg-nav` in both themes. Disabled controls use 45% opacity and are exempt.
- **Accent `accent`** (Steam-like sky blue) means selected, linked, focused, and in-progress. **Green `install`** means one thing: the verb that changes the game's state, with `on-install` text. Never use install green for decoration or for success text; success text uses `ok`.
- **Status colors** `ok`, `warn`, `danger`, `info` appear only as a badge (word plus icon on a `*-bg` fill), a dot beside a row, or a progress bar. Status is never carried by hue alone: `ok` and `danger` differ in lightness and every badge has a label.
- **Borders and lines.** `line` is for decoration (dividers, card edges). Anything that must be seen to be used (input, switch, unselected chip) uses `line-strong`, which holds 3:1.
- **Focus.** A solid 2px `accent` ring (`shadow-focus`), on keyboard and gamepad focus, offset 2px. It holds 3:1 on every surface. Gamepad focus uses the same ring, larger hit area, no hover.
- **Corners.** Near-square like Steam: `radius-sm` 2 (badges, bars, chips), `radius-md` 4 (buttons, inputs, capsules, rows), `radius-lg` 8 (dialogs, sheets). `radius-pill` only for the switch.
- **Elevation.** Borders first, shadow second. `shadow-card` only on a hovered capsule, `shadow-pop` on dialogs, menus and drawers. No glows, no gradients except the striped art placeholder.
- **Type.** Hanken Grotesk for UI, JetBrains Mono for data. Scale: `title-hero` 32, `title-page` 22, `heading` 16, `body` 14 (`body-touch` 16), `label` 13, `meta` 12, `eyebrow` 11 uppercase for nav and section labels.
- **Spacing.** 4px base: `space-1`..`space-8`. Page gutter `space-5` on desktop, `space-4` on phone.
- **Motion.** 120ms for hover lift and color; 240ms collapse of sidebar to rail; 280ms drawer and 300ms sheet, expo-out. Honor reduced motion by dropping to instant.

## Layout and density

Pick the layout from the container width (see the Layout section), and the density from the input device. They are independent: a 1280x800 handheld uses the wide layout at touch density.

## Iconography

Lucide, through `freya_icons::lucide`, 16px in rows, 22px in rail and tabs, 18px in large buttons. The previews draw simple stand-ins; each stand-in carries its Lucide name in `data-lucide`, and the `.freya.json` contract names the real icon per component. Never transcribe SVG paths.

## Logo

There is no Reclaw mark yet. Set the name "Reclaw" in `Hanken Grotesk` 700; do not invent a symbol. Game art always comes from the catalog; when it is missing, show the striped placeholder with the art role in mono (CAPSULE 3:4, HERO 16:5), never a generated image.

## Deck mode

For handhelds and TVs the same system has a controller-first shell: big-art tiles on shelves, a focus ring with a glow, slide-in menu and Quick Access panels, glyph prompts that follow the controller in use, and a Play button that becomes Resume and Stop while an app runs. It is a mode on the same components, not a theme. Deck is dark only and uses the `deck-*` tokens (safe zone, tile and row sizes, type scale, `deck-bg`, `deck-scrim`, `focus-glow`). Over art, text may only be `ink` or `ink-muted`, set on `deck-scrim`. The design, its decisions and what is not built are in the Deck mode section; the machine-readable spec is `reclaw.freya.json`.

## Using this system with Freya

Start from `freya/theme.rs.txt` (save as theme.rs) (generated from `tokens.json`) and the Freya handoff section. The machine-readable contract is `reclaw.freya.json`; where it disagrees with this text, the JSON wins.
