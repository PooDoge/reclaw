# 0021 Deck tiles carry the whole game id, a game is on one Library shelf, and Deck has a Catalog

- status: accepted
- date: 2026-10-07
- spec: ../specs/routing.md

## Context

On the real catalog, Deck mode misbehaved in three ways that the fixtures (game ids 1 to 6) never showed:

* A tile's focus id was `TILE | shelf << 20 | game` in a `u32`. App ids are FNV hashes of the app's key (`catalog_data::IdMap`), so
  any of their 32 bits can be set. The game id overwrote the kind tag and the shelf number: most tiles decoded as no game at all
  (A did nothing), and two copies of one game on two shelves could get the same id, so both were focused at once.
* Every installed game was on *Continue* and again on *All apps*, so the Library showed each of them twice.
* The Catalog tab was a placeholder ("Community app lists will appear here").

## Decision

* **`FocusId` is a `u64`.** The low word keeps the kind tag and the small index as before; a tile's game id sits whole in the high
  word. Every other kind leaves the high word at zero, so their decoding is unchanged.
* **The Library has *Continue* (installed or running) and *Not installed*.** A game is on one shelf only; by system, *Not installed*
  is split per system as *All apps* was.
* **The Catalog tab is a shelf per system** of `catalog::catalog_entries` (the same join the desktop Catalog uses), drawn by the
  Library's `HomePage` and focused through the same tile ids. `DeckView::game` and `Frame::game` fall back to the catalog, so a
  project outside the library opens its page, installs, and can be added from Options.

## Rejected

* **Tiles numbered by position (shelf, index) instead of by game.** It fits in 32 bits, but focus memory and the hint of which game
  is focused would then follow a slot, not a game: a game finishing its install moves between shelves and focus would land on
  whatever took its place.
* **Hashing the game id down to 20 bits.** Collisions would be rare but real, and silent.
* **Keeping the duplicate shelves, as Big Picture does with "Recent".** Continue here is "installed", not "recent"; there is no play
  history yet to make a separate recent shelf mean something, and the duplicates read as a bug.
* **A grid page for the Catalog** like the desktop's. Shelves are what the pad already moves through, and the platform chips the
  desktop has are what the per-system shelves replace.
