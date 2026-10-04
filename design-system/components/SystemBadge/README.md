The mark of the system a game was recompiled from ("N64", "PS2"), as a small chip. Rust: `reclaw_ui::components::SystemBadge`; names, order and grouping come from `reclaw_games::project::Platform` (one table, tested), the filter and sort by system from `reclaw_ui::systems`.

Neutral in color on purpose, like the controller glyphs: the letters carry the meaning, so it reads the same in both themes and for everyone, and it never competes with a status color. A 1px `line-strong` border, `ink-muted` mono text; `over_art` gives it a solid `bg-base` fill and `ink` text so it stays legible on any picture; `large` is the Deck size (hint-size type). A game of no known system draws nothing.

Where it shows: Library rows, capsules, the hero, catalog cards, Deck tiles (large, over art), and the system filter's picker. Deck's Home can group shelves by system (Settings > Library > Sort).

Don't: color it by system, or use it for status.
