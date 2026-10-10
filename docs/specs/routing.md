# Routing

- last-verified: 2026-10-07
- owner-paths: reclaw-ui/src/nav/**, reclaw-ui/src/pages/**, reclaw-ui/src/deck/app/routes.rs, reclaw-ui/tests/ui/nav_stage.rs, reclaw-ui/tests/ui/deck_routes.rs

Which page shows, how the user got there, how Back works, and how one page gives way to the next. Both interfaces use the same
routes: the desktop draws them as pages, Deck mode draws them as screens.

## The rule

**The router is the truth about which page is showing.** Nothing keeps a second copy. Deck's reducer moves itself and *asks* the
router to follow (`Effect::Navigate`, `Effect::Back`, handled by Deck's `Dispatcher` and never sent to the host); when the router
moves for any other reason (the mouse's back button, a link, Quick access) the reducer *follows* it (`DeckState::follow`). The two
keep out of each other's way by remembering the route each last asked for.

## Pages

`Route` (`nav/route.rs`) is one enum under one layout, `AppLayout`: Library, Catalog, Game, Install, GameSettings (and a section),
Mods, ModDetail, Downloads, Settings (and a section). `freya-router` derives each route's path from it, so a route is a link, a saved
location, or a command-line argument (`reclaw --open /game/4`). There are no nested layouts: a section such as Mods is a set of
sibling routes, so a transition never animates a layout around an outlet.

`Route::kind()` says what a page is: a **root** (a tab), a **detail** page, a **form** (kept out of the recent pages), or settings.
`Route::up()` is the page one level above, used when there is no history to go back through.

## Moving

* `Nav::open(route)`: a root **replaces** the current page (tabs do not grow the back stack); anything else is **pushed**. Opening the
  page already showing does nothing.
* `Nav::back()`: closes the topmost **layer** (a dialog or menu registered with `open_layer`) first; else goes back in history; else
  goes `up()`; else returns false.
* `Nav::forward()`. Mouse side buttons, Alt+Left / Alt+Right, and the Back / Forward keys do these (`NavInputExt`). Escape is Back on
  the desktop; Deck's reducer owns Escape and the pad's B (it closes its own menus first), so Deck passes `escape: false`.
* `Nav::recents()`: the pages visited lately, newest first, without the current one. Kept apart from the back stack (the router's
  history cannot be read back) and fed on every change; forms are skipped. `nav::title` names a page for a list.

A route that is a Deck screen the pad cannot show (the Mods tab is a placeholder in Deck mode, and a single mod's page shows that
tab) still routes; the reducer shows what it has. Deck's Catalog tab is a shelf per system of every catalog project; a tile opens
the game's page whether or not the game is in the library (the page offers Install and, in Options, *Add to library*). A game page
stays while the library or the catalog knows the game, and goes back to the tab when neither does.

Deck focus ids (`deck/state/ids.rs`) carry a tile's whole game id in their high 32 bits: app ids are 32-bit hashes, and an
encoding that packed them beside the kind tag and shelf number lost bits, so a press on a real game did nothing and one game could
be focused on two shelves at once.

## Transitions

`RouteStage` draws the current page and, while it animates, the previous one. `StageState` (pure) decides what is on stage;
`PageMotion` tells a page how far its entrance has gone, so a page can choreograph itself (the Game page brings its banner in first
and the sections after). `TransitionConfig` is the policy: an intensity (off, subtle, standard, cinematic), a default style (slide,
fade, rise, zoom) and "pages pick their own style", **separately for the desktop and for Deck mode**, plus a global *Reduce motion*
that cuts everywhere. A route may ask for a style (`Route::enter_style`); the setting "Pages pick their own style" lets it. The
settings page edits all of this (Settings > Motion), and `RECLAW_MOTION` pins it for tests and screenshots.

## Tests

`tests/ui/nav_stage.rs` (the router, Back, layers, mouse buttons, transitions), `tests/ui/deck_routes.rs` (the two-way sync),
`tests/ui/recents.rs`. The pure parts (`recents`, `meta`, `stage_state`, `titles`, `transition/*`) have unit tests beside them.
