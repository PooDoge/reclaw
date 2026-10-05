# 0012 The program has no sample data

- status: accepted
- date: 2026-10-05
- spec: ../specs/catalog-format.md

## Context

Until now the screens were built over invented games ("Starfall 64", "Dino Rush"), a scripted fake download, invented mods and hard-coded
"collections", so that layouts could be looked at before any real data existed. The real catalog (232 projects) and the original
launcher's source were available all along, and every placeholder meant a second pass to replace it, hiding the problems that only
real data shows. The first run against the real catalog found two at once: a hook called only when a list was non-empty (a panic
the moment the catalog finished loading) and a filter chip whose count wrapped onto two lines.

## Decision

The program contains no sample data. It shows the user's library (`apps.json`) and the community catalog, loaded from the network and
saved to disk, and says plainly when something is not available (no catalog yet, no network, an installer that is not built). What had
only existed for the sample data is gone: the scripted installer, the live-sample switch, the invented mods and the three
"collections". Tests keep a small set of invented games (`fixtures`, compiled for tests and the `fixtures` feature only), because tests
must be deterministic and offline; the same tests also run against the real catalog when `QUIVER_CATALOG_DIR` points at a checkout.

A game's number (saved with favorites and per-game settings) is the 32-bit FNV-1a hash of its identity key, with the later of two
colliding keys taking the next free number, so it is the same on every start and independent of the order the catalog lists things.

## Rejected

* **A demo mode behind a flag.** It keeps two programs alive, and the one the flag hides is the one that is never looked at.
* **A bundled snapshot of the catalog for first start or tests.** The catalog has no licence, so copying it into this repository or a
  release is not ours to do, and a snapshot is stale the day it is taken. The first start shows an empty, explained catalog until the
  network answers; the cache makes every later start instant.
* **Recorded network responses for tests.** The same licence and staleness problems; the local test server serves invented but
  correctly shaped files, and the real catalog is read from a checkout when there is one.
* **Sequential numbers for games.** They would change when the catalog's order does, orphaning saved favorites.

## Consequences

An empty library is the real first-run state, so every screen has to be good empty. Pressing Install, Play or Update says it is not
built yet; adding a game to the library works today. The design-system previews are static mockups and still use invented names.
