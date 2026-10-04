# 0009 Reclaw reads and writes Quiver's catalog format

- status: accepted
- date: 2026-10-04
- spec: ../specs/catalog-format.md

## Context

Reclaw's catalog so far was sample data in a shape of our own (`ProjectInfo`). The Quiver launcher already has a community catalog of
232 real recompilation projects, with artwork addresses, repositories and mod sources, published as JSON and kept up to date by its
maintainers. The aim is to start from that data, to be able to merge a catalog of our own with it later (projects Quiver has not listed),
and to bring the launcher's behaviour over to Rust before improving on it.

## Decision

A crate, `reclaw-catalog`, reads and writes Quiver's four documents with the same keys, the same lenient and strict modes and the same
normalisation. Anything Reclaw wants to add goes in **one optional object, `reclaw`**, on an app entry. A catalog we publish is a valid
Quiver catalog; one of theirs is a valid Reclaw catalog with nothing extra.

## Rejected

* **A schema of our own and a converter.** Two shapes to keep in step, and a merge with their catalog would be a translation every time it
  is fetched. Every field Quiver has would be re-invented, and the places we would differ (identity, dedupe, normalisation) are the ones
  that make "the same app" mean the same thing in both catalogs.
* **New fields beside Quiver's, at the top of the entry.** Quiver ignores unknown keys, so it would work today, but a field Quiver adds
  later (`summary`, `description` are plausible names) would collide with ours with no way to tell whose it is. One namespaced key cannot.
* **A separate file of extras keyed by repository.** This is the right answer for entries in a catalog we do not own (see below), but as
  the only mechanism it splits one app across two fetches that drift apart.
* **A date and time crate for the platform index.** The format needs one question answered, "is this instant before that one, give or take
  five minutes", and `time`/`chrono` bring a calendar, zones and formatting. A parser for one fixed shape, checked against Python's
  epoch values and against leap days and offsets, is smaller than the dependency's surface. It refuses a time without an offset.

## Consequences

* The `reclaw` block can only be added to lists we publish. To give the 232 existing entries hero art or a description we need an **overlay**:
  a Reclaw list whose entries carry only a repository and a `reclaw` block, merged by identity key. Not built; the identity keys it needs exist.
* Sharing one `apps.json` between Quiver and Reclaw loses the `reclaw` blocks whenever Quiver saves (it writes from its own model). Import a
  copy instead; the blocks come back from the catalog.
* We inherit Quiver's quirks where they are part of the format (an empty icon string wins over later icon keys; unknown mod providers are
  kept) and drop them where they would differ by operating system (file-name characters). The spec lists each.
* Dates are the only thing here reimplemented from scratch rather than taken from a library; its tests are the evidence. The one new
  dependency is `sha2` (the content hash and safe cache names).
