# Search (desktop)

- last-verified: 2026-10-10
- owner-paths: reclaw-ui/src/search/**, reclaw-ui/src/desktop/search.rs, reclaw-ui/src/components/search_toggle.rs, reclaw-ui/src/desktop/pages/common/results.rs, reclaw-ui/tests/ui/desktop_search.rs

One search button for the desktop interface. It searches the tab that is showing, opens into a bar without moving anything, and closes
back into the button. Decision record: ADR 0024. The Deck has no search.

## Which tab is searched (`search/scope.rs`)

`SearchScope` is Library, Catalog, Mods or Settings: the top bar's tab that is lit. A game's page searches the Library, a mod's page the
Mods; Downloads and an app's properties have no tab and show no floating button (the wide top bar keeps its button, which searches
the Library from them). The placeholder says where: "Search your library", "Search the catalog", "Search Starfall 64 mods" when a game
is chosen on the Mods tab, "Search settings".

## Open, close, send (`search/state.rs`, `desktop/search.rs`)

* **Focus decides.** Clicking the button focuses the bar's input and the bar opens; losing focus (a click elsewhere, Escape, Tab)
  closes it. Nothing else opens or closes it.
* **Enter, or the magnifier while open, sends.** The query becomes the tab's search, the bar closes, and if the search came from a page
  that is not the tab (a game's page), that tab is opened. Sending empty text clears the tab's search.
* **Text is kept.** Closing without sending keeps what was typed (the draft) for that tab; reopening within 120 seconds
  (`DRAFT_KEPT_SECS`) puts it back. After that the draft is forgotten, unless it is the search that is running, which is kept as long
  as it runs. Each tab has its own query and draft.
* While a tab's search is running, the closed button carries an accent dot.

## How the button looks (`components/search_toggle.rs`)

* **Wide**: in the top bar, right of the tabs, a 32-unit square (36 with a pointer density, 44 on touch). Open, the bar is 320 wide.
* **Compact and phone**: the same button floats at the page's top right (16 in from the page's edge, 16 from its top), above the page,
  on a tab page only. Open, it covers the page's width less the gutters. Each tab's header leaves room for it (`floating_search_reserve`).
* The button's own box never changes size: the bar is absolute, anchored at the button's right edge, and grows to the left over its
  neighbours (`Layer::Relative(1000)`, below dialogs). The magnifier stays at the middle of the button's square, closed, open
  and while sliding: it is the bar's right end, which does not move. The width slides in 240 ms (expo out), instant with reduced motion.

## What a search shows

Every word must appear (any order, any case; `search/text.rs`). A results line says "3 results for “hd”" (or "in the catalog" when
nothing is found) with **Clear search**; the matched words are drawn in the accent colour.

* **Library**: the games whose title, project or tags match, within the sidebar's filter.
* **Catalog**: the projects whose title, project or tags match, within the system chip.
* **Mods**: see `mods.md` (the shelves give way to the matches, 20 a page).
* **Settings**: the rows of Reclaw's own settings whose label, description or choices match, under their sections' titles, working
  in place (`filter_schema`). A section's title matching shows all its rows.

When nothing matches: "Nothing in the catalog matches “x”", a hint, and **Clear search**.

## Not built / not verified

* The caret is at the start of the text when a kept draft comes back (Freya's `Input` starts its caret at 0).
* Searches only what Reclaw has already listed; it never asks a site.
* Verified on the headless renderer and by snapshots only; the slide was not watched on a real window.
