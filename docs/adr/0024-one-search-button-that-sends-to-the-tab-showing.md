# 0024 One search button that sends to the tab showing, and Mods shelves from the sites' own orders

- status: accepted
- date: 2026-10-10
- spec: ../specs/search.md

## Context

Each desktop tab drew its own search field in its header; on a narrow window it took a full row above the content. The Mods tab
was one list in the order one site gave, with no way to see what is new, recently updated or well liked.

## Decision

* **One `SearchToggle`** in the frame (the wide top bar, or floating at the page's top right below that) searches the tab that is
  showing (`SearchScope`). Its box never changes size; the open bar is absolute and slides out to the left over its neighbours.
* **Focus opens and closes it; Enter sends.** Results follow the sent query, not each keystroke. What was typed and not sent is
  kept per tab for two minutes (`SearchModel`, pure and tested).
* **Mods shelves come from the sites' own orders.** The host asks each source in four orders and keeps each mod's place in each
  (`ModRanks`); a shelf shows the mods its order listed.

## Rejected

* **Filtering as you type.** It is what the old fields did, but with a bar that closes on blur the results would change under the
  person while typing and stay changed after an accidental click away; sending makes the closed button's state ("a search is
  running") unambiguous.
* **Explicit open state set by the button's press.** Two sources of truth (pressed and focused) drifted in the first version: a
  click outside closed focus but left the bar open. Focus is the one Freya already tracks.
* **Growing the button in the layout** (a width animation on an in-flow box). It moves the tabs or the header with every frame of
  the slide, which is the layout shift asked to avoid.
* **Shelves by sorting the one listing locally** (by date, by rating). One request per source instead of four, but the first 60 most
  downloaded mods are not the newest or best rated: a site's "newest" shelf would be its oldest popular mods.
