# 0001 Routing with freya-router, shared by both interfaces

- status: accepted
- date: 2026-10-04
- spec: ../specs/routing.md

## Context

The app needs pages (library, catalog, a game, a mod, settings ...), a way back, animated transitions that differ by interface and
by page, mouse back and forward buttons, and a controller B button that does the same thing. Deck mode started with its own
screen stack inside its reducer.

## Decision

One `Route` enum under one layout, driven by `freya-router`. The router is the single truth about which page is showing; Deck's
reducer asks it to move and follows it when it moves for another reason. Roots (tabs) replace, other pages push; Back closes
overlays first, then goes back in history, then up. A recents list is kept beside the router.

## Rejected

* **A second navigation stack inside Deck's reducer** (what existed). Two truths drift: the mouse's back button moved the router and
  not Deck. The reducer keeps its focus and screen state, but not the question "which page".
* **Nested layouts per section** (Mods as a layout with an outlet). A transition would have to animate a layout around an outlet,
  and Back stops meaning "the previous route". Sections are sibling routes instead.
* **Our own route enum and a `State<Route>`** with no router. It would have worked, but gives up paths: links, `--open /game/4`, a
  saved location, and a site map for free.
* **Reading history back from the router for the Recent menu.** It cannot be read, and the back stack is the wrong shape anyway
  (going back does not forget a page). A separate `Recents` list is fed on every change.

## Consequences

Every page is a component that takes its parameters from the path, so it cannot be given anything else; what it needs comes from
context (`ShellCtx`, `DesktopUi`). The transition code lives in one place (`RouteStage`). Deck pages are routes drawn from a shared
frame. A router upgrade is a migration of one crate's API, not of our navigation model.
