# 0022 The host says which games take mods, and the Mods tab's game is frame state

- status: accepted
- date: 2026-10-10
- spec: ../specs/mods.md

## Context

The Mods tab listed every installed game's mods in one list, with no way to see one game's, and a game's page sent the person to
that whole list. Narrowing to a game needs the list of games that take mods; the screens only had the mods, each with its game's id.

## Decision

* **The host sends the games that take mods** (`AppAction::SetModdable`, ids), next to the mods, from the same `moddable()` that
  decides which games are listed. They live in `AppState` on the Mods channel; `mod_games` joins them with the library for titles
  and counts.
* **The chosen game is the desktop frame's state** (`DesktopUi::mod_game`), beside the site chip and the search, and a game's page
  sets it (with the site chip on All and the search cleared) before opening `Route::Mods`.

## Rejected

* **Working the games out from the mods' `game_id`s.** No new state, but a game whose sites have listed nothing yet, or did not
  answer, would be missing from the choices and its page would have no link, which is the case the person most needs to see.
* **A game in the route (`/mods/:game`).** Back and Recent would remember the game, but `Route::Mods` is a root tab the Deck shares
  and the nav bar opens without a game; the site chip and the search are already frame state that survives moving between pages, and
  the game behaves the same way.
