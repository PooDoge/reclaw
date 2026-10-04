The Steam-style library tile: 3:4 portrait art, title, recompilation project, status badge. Freya: a `Card` with fixed width `capsule-w` (168), image via `image()`; `Content::Flex` is not needed because the card is a plain column.

Consumer provides: art (catalog supplies it; fall back to the striped placeholder with the title set in mono, never a generated logo), title, project, status. Hover lifts 2px, draws an accent border and, if installed, reveals the Play button over the art; on touch there is no hover, so Play appears in the hero, not the tile.

Grid: columns `repeat(auto-fill, capsule-w)` on wide, 4 fluid columns on compact, 2 on phone with `fluid` (width 100%).
