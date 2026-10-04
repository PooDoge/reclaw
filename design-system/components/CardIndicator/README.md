What a Deck card shows over its art while its game has background work. Rust: `reclaw_ui::deck::CardIndicator` (in `deck/widgets/card_progress.rs`), built from the game's `Indicator`.

* A chip at the top left with the icon and text (large type, solid `bg-base` fill), and a bar along the bottom edge (`CARD_STRIP_H` = 8, a Rust constant, not yet a token).
* The bar **fills** when the size is known, **slides** (a third of the track, 1.4s, in-out) when it is not, is **full and green** when finished, and is **absent** when only an update is waiting or the job failed.
* Both sit inside the art's clip, so they scale with the focused card and keep its rounded corners.
* Only cards that need the sliding animation run it; a finished card is static.

Reduced motion: the sliding bar should become a static half-filled bar. Not implemented yet.
