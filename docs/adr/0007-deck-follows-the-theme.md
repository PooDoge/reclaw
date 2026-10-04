# 0007 Deck mode follows the theme

- status: accepted
- date: 2026-10-04
- spec: ../../design-system/deck-mode.md

## Context

Deck mode was dark only. Its page background, the scrim over art, the focus glow and the dim behind modal menus carried the same
value in both themes, while the text colours (`ink`, `ink-muted`) followed the theme. Switching to the light theme therefore left a
near-black page behind near-black text: the tab strip, shelf titles and hints were hard to read, and the focused row of a menu
(drawn as `ink` with `deck-bg` text) became dark text on a dark highlight, on the desktop's popup menus too. The first record of the
decision (`deck-dark-only` in the contract) argued for TV and OLED viewing and for halving the contrast matrix.

## Decision

Deck mode follows the theme like everything else. `deck-bg`, `deck-scrim` and `focus-glow` have a value per theme (daylight: the page
grey of `bg-base`, the same grey at 82% over art, and the daylight accent at 45%). Two tokens are new: `deck-dim`, the dark veil behind a
centered menu, a confirmation or a notice's details (dark in both themes, so the light panel stands out in daylight), and `deck-dim-ink`,
the light text that sits directly on it (a menu's title). The contrast checker covers every new pair in both themes, for the worst
image under a scrim (white under a dark one, black under a light one).

## Rejected

* **Keep Deck dark and pin it to the midnight tokens whatever the setting says.** One theme to design for, and right for a TV in a dark
  room. But the person who chose the light theme sees their choice ignored in half the app, and a Deck-sized window on a desktop is
  exactly where the light theme gets used.
* **Dropping the `deck-*` colour tokens and reusing `bg-base`, `scrim` and the accent.** Fewer tokens, but the roles differ: the scrim
  over art has to hold 4.5:1 against the worst picture, the glow is an alpha of the accent, and the dim is darker than `scrim`.
  They would be re-derived by hand in each place.
* **A light veil behind menus in daylight.** Keeps `ink` readable on it without `deck-dim-ink`, but the white panel then barely
  separates from the page; the dark veil is how every other light interface does modals.

## Consequences

A light Deck is brighter in a dark room and, being untuned, may glare on an OLED handheld; the setting is still the user's. In daylight
the art placeholders (`bg-raised`) sit close to the page grey, so a game without art is a pale tile; real art covers it. The focus
glow is faint on a light page, which is why the 3px accent ring, not the glow, carries focus. The Deck previews in the design system
now show both themes. A bug that the dark theme had hidden surfaced with the white panel: the main menu's scrim was painted over the
panel's own background (siblings were not painted in order), fixed with `Layer::Relative(-1)` on the scrim.
