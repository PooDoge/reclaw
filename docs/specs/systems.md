# Systems: the console a game came from

- last-verified: 2026-10-07
- owner-paths: reclaw-games/src/platform.rs, reclaw-ui/src/systems.rs, reclaw-ui/src/components/system_badge.rs, reclaw-ui/tests/ui/systems.rs

Every game is a recompilation of a game from some system. Reclaw shows which, and lets the user narrow and order lists by it.

## The model

`Platform` (`reclaw-games`) lists twenty-one systems plus `Other`: Nintendo's NES to 3DS, Sony's PlayStation to PSP, Xbox and Xbox 360,
Sega's Genesis to Dreamcast, then Arcade, PC and Mobile (the community catalog has apps for each). Each has a name, a short mark of at
most four characters ("N64", "GCN", "PS2"), a maker, a kind (console, handheld, arcade, computer, phone), and a **rank**: the order of the `ALL` array, grouped by
maker and oldest first within a maker. Adding a system means adding a variant, an entry in `ALL`, and each `match`; the compiler
points at all of them. The saved names (`n64`, `ps2`, `gba`, `other` from the first version) do not change; new systems were added without reordering saved values.

`GameEntry::platform` is set from the catalog project, or from the tags (`Platform::from_tags`). Catalogs tag generously, so the first
tag is not the answer: a tag naming one system (`ps2`, `x360`) beats a brand tag (`playstation`, `xbox`), which beats a tag for where the
game runs today (`pc`, `mobile`); ties go to the tag listed first. `Platform::from_tag` still answers for one tag. See `catalog-format.md`.

## What the user sees

* A neutral **badge** with the mark on library rows, capsules and Deck tiles (no brand colours; the letters carry the meaning, as
  with the controller glyphs). None for `Other`.
* **Desktop Library:** an *All systems* chip lists the systems the library has games for, with a count each, and narrows the list;
  a *Sort* chip chooses the order. The hero shows the first game that passes the filters. The Catalog's system chips are in system order.
* **Sort** is one setting, Library > *Sort games by*, shared by the Library, the Catalog and Deck mode: **Added** (the order the games
  joined; the default, so nothing reorders on its own), **Title**, or **System** (grouped, oldest first, A to Z within a system).
* **Deck Library:** *Continue* (installed or running) over *Not installed*; a game is on one shelf only, and an empty shelf is left
  out. Sorted by system, *Not installed* becomes one shelf per system titled with the system's name.
* **Deck Catalog:** always one shelf per system, system order, A to Z within a system.

## Where the logic is

`systems.rs` is pure: `Sort`, `sorted`, `on_system`, `systems_in`, `grouped`. The filter on the desktop is `pages/library/filter.rs`;
Deck's shelves are `deck/state/view.rs::shelves`.

## Not built

A system filter in Deck mode (the shelves group instead); icons of the systems themselves rather than their marks.
