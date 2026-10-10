Primary navigation: Library, Catalog (community app lists, from Quiver), Downloads, Mods (Thunderstore, GameBanana). Three forms of one component, chosen by the container width, not the OS:

- `top` at ≥1100px: Steam-style uppercase eyebrow items with a 2px accent underline on the active one.
- `rail` at 720-1099px: 64px icon rail, 44px hit targets, 3px accent bar on active.
- `bottom` under 720px: 56px tab bar, icon over 11px label, 44px minimum targets, active in accent.

Freya: `rect()` rows/columns of `SideBarItem`/`FloatingTab`; the icon-only rail needs a `TooltipContainer` per item. Downloads shows a count badge while the queue is active.

Search: one `SearchToggle` at the right of the `top` bar (32px square, opens to 320px leftwards over Recent and Deck mode). Under the `top` form the same button floats at the page's top right instead of living in the rail or tab bar (see SearchToggle).
