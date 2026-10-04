Big-art game tile for Deck mode. Rust: `reclaw_ui::deck::DeckTile`.

The title is always visible (placeholder art has none baked in). Focus scales the tile 1.06, draws a 3px solid accent ring and a `focus-glow` halo; the badge appears on focus or while the app runs, in space reserved so focusing never reflows the shelf. Running takes priority over install state.

Consumer provides the game and whether it is focused (from `DeckState`, never from hover). The ring is drawn only after gamepad or keyboard input.
