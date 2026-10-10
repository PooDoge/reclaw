# Catalog and library format (Quiver-compatible)

- last-verified: 2026-10-10
- owner-paths: reclaw-catalog/src/**, reclaw-catalog/tests/real_catalog.rs, reclaw-games/src/platform.rs, reclaw-sync/src/**, reclaw-sync/tests/sync.rs, reclaw-ui/src/catalog_data.rs, reclaw-app/src/**

Reclaw reads and writes the same four documents as the Quiver launcher, so the community catalog works unchanged and a catalog
Reclaw publishes can be read by Quiver. Since 2026-10-09 the community lists are **frozen**: Quiver 3.5 browses quiverlauncher.com
instead, and the lists stay in place "so existing installs keep working". Reclaw still reads them as its catalog and reads the site
beside them (spec: community, ADR 0023). The crate is `reclaw-catalog`: plain data in and out, no network, no disk, no window.
Why this and not a schema of our own: ADR 0009.

## The four documents

| Document | Who writes it | Read | Code |
|---|---|---|---|
| `apps.json`, the library | the launcher, for one user | **strict**: any entry of the wrong shape is an error and nothing is returned | `library.rs` |
| a list file (`Nintendo.json`) | a catalog author | **lenient**: an unreadable entry is skipped and reported, the rest are used | `list.rs` |
| `index.json`, the community index | the catalog maintainers | case-insensitive keys, `version` must be a number, no lists is an error | `index.rs` |
| `platform-index.json` | a generator, from the hosting service | **all or nothing**: one bad entry rejects the file | `platform_index.rs` |

The library is strict because what is read is written back: a copy with an entry quietly missing, saved over the original, is lost data.
A catalog list is somebody else's file and one bad entry must not hide 231 good ones.

## One app

`AppEntry` serves the catalog and the library. Keys are lowerCamelCase and case-sensitive; unknown keys are ignored on read.

* **What the catalog says**: `name`, `project`, `repository` (`owner/name`), `repositorySource` (`github`, the default, or `gitlab`),
  `folderName`, `appIconUrl` (older files: `gameIconUrl`, `customDefaultIconUrl`), `tags`, `filesToAdd`, `releaseAssetFilter`, `mods`,
  `catalogId`.
* **What belongs to this user** (library only): `installPath`, `preferredVersion`, `skippedUpdateVersion`, `customDisplayName`,
  `autoUpdate`, `deferUpdateTracking`, `linuxRunner`, `linuxPrefixPath`, `linuxProtonPath`, `linuxCustomLaunchCommand`.
* **Quiver 3.5's link to quiverlauncher.com** (library only): `catalogEntryId` (the site entry the app was added from or last linked
  to) and `catalog` (`{name, project, appIconUrl, tags}`, what the site last set, so a later change of the site's does not overwrite
  the user's). Reclaw reads and writes both unchanged (`snapshot.rs`); it uses `catalogEntryId` to link the app (spec: community) and
  does not yet move a field with the site. Before this, saving the library dropped them, which unlinked a library Quiver shares.
* A **manual** app has no `repository`. It cannot be pinned, skipped, auto-updated or filtered; reading forces those off.
* **Identity** is `manual:<folder>` or `<source>:<repository>`; the **tile** (instance) key adds `:<folder>`, because one repository can hold
  several games (told apart by `releaseAssetFilter`) and each has its own folder. Keys compare ignoring case but keep their spelling.
  A document with two entries for one tile keeps the first.
* **Normalisation** is applied when reading and again when writing: tags are trimmed, lower-cased, de-duplicated (first place wins);
  `filesToAdd` are plain file names (no separators, no `.`/`..`, no characters a file name cannot hold), de-duplicated ignoring case;
  `mods.path` is `/`-separated with no `.`/`..` (otherwise it is dropped whole); `releaseAssetFilter` is trimmed and blank means none.
* **Writing** the library follows Quiver's key order: `name, folderName, installPath, appIconUrl` always (explicit nulls), then for a hosted
  app `repository, preferredVersion, skippedUpdateVersion`, then the optional keys, then `tags, catalogEntryId, catalog, filesToAdd,
  releaseAssetFilter, mods`.
  Defaults are not written (`autoUpdate` only when true, `linuxRunner` not when `auto`). A catalog entry leaves out the user's fields.

## List, index and platform index

* **List**: `name`, `description`, `version` (a string; a number is ignored), `iconUrl` (absolute http/https only, else none),
  `featuredTags`, `preferredTagFilters`, `hiddenTagFilters`, `apps`. Older files with `standard`/`experimental`/`custom`, or a bare array,
  still read. No `version`: a SHA-256 of the apps' content stands in (`list::content_hash`), independent of order, ignoring `mods`.
* **Index**: `version`, `lists[]` (`id`, `remoteLocation`, or a web `location`), `platformMetadataUrl`. A list's id names its cache file and
  Quiver uses it unchecked; `IndexSource::cache_stem` makes any id a safe file name, and two ids never share one.
* **Platform index**: per `(provider, repository, preferredRelease)`, the release tag looked at and the asset names it held. Valid only if
  `formatRevision` is 1, every entry has `provider` exactly `github`/`gitlab`, `selectionRevision` 1 or 2, no entry was validated more than
  five minutes after `generatedAt`, keys are unique, and the size limits hold (16 MiB, 10 000 entries, 10 000 assets, 2048 characters a name).
  GitHub repositories match ignoring case, GitLab exactly. An entry is **fresh** only at revision 2 and under 24 hours; revision 1 is usable,
  never fresh. It is evidence about what exists, never an instruction to download. Timestamps need an explicit offset (`timestamp.rs`).

## Systems

`AppEntry::system()` is the author's `reclaw.platform` if it names a system, else `Platform::from_tags`. Catalogs tag generously (a PlayStation 2
game carries `playstation`, `ps2`, `playstation 2`), so the first tag is not the answer: a tag naming one system beats a brand tag
(`playstation`, `xbox`), which beats a tag for where the game runs now (`pc`, `mobile`). An N64 game ported to PC is an N64 game; a Club
Penguin recreation is a PC game. The six systems the community catalog needed and Reclaw lacked (Xbox 360, Wii U, 3DS, Arcade, PC, Mobile) were
added; before, 33 Xbox 360 apps would have shown as the original Xbox and every PC and mobile app as *Other*.

## Reclaw's optional block

Under one key, `reclaw`, so it cannot collide with a field Quiver adds later. Quiver ignores it. Every field is optional and a damaged
block is dropped without failing the entry. `summary`, `description`, `heroUrl`, `capsuleUrl`, `screenshots[{url, caption}]`,
`videos[{url, title, thumbnail}]`, `requirements`, `platform`, `capabilities`, `links{}`. Quiver's `catalogId` is kept and written back.

## Where this is deliberately not Quiver

* Invalid file-name characters are the **Windows** set everywhere (Quiver asks the OS, so the same file parses differently on Linux and Windows).
* A timestamp without an offset is refused (it would mean local time, a different instant on each machine).
* Index ids are sanitised for use as file names.
* An icon address is returned in the parsed (canonical) spelling.

## Checked against the real catalog

`reclaw-catalog/tests/real_catalog.rs` reads a checkout of the community catalog (`QUIVER_CATALOG_DIR`; they pass without it, saying so).
On 2026-10-04 that is 4 lists and 232 apps: none skipped or merged away, every entry writes back to the same JSON apart from one hand-edited
duplicate tag, all 232 have platform metadata, all 232 have a system. The catalog's own data is **not** copied into this repository (no licence).
Reading the C# is how the rules above were found; nothing was run against Quiver itself.

## On the screens (`catalog_data`, `reclaw-app`)

`catalog_data::load(catalog, library)` turns the sync's apps and the library into the `ProjectInfo`s of the Catalog and game page and the
`GameEntry`s of the Library. Title, project (the port's name), tags and system come from the entry; the latest release tag and its page
from the platform metadata; the picture is the `reclaw` block's capsule if the entry has one, else `appIconUrl`; summary, description,
screenshots, requirements and launch capabilities exist only where a `reclaw` block gives them (none of the 232 do yet), and the game
page fetches the project's README itself. A game's number is the hash of its identity key (ADR 0012).

The host (`reclaw-app/src/host.rs`) loads the saved catalog before the first frame, refreshes in the background (one refresh at a time),
keeps the library file (`Add to library`, `Remove from library`; a library that cannot be read is left untouched and read-only), opens
links (public https only), and answers every request for something not built yet with a note that says so.

## Not built

Writing a *catalog* to disk (only the library text is produced), the comparison that decides what is new or changed against a library
(Add / Merge / Replace), classifying a release's assets by platform, an **overlay** list for adding `reclaw` blocks to entries in a catalog
we do not own, and the files and network around all of this. See `docs/quiver-parity.md`.

## Loading and keeping it (`reclaw-sync`)

`CatalogSync::refresh` fetches the index, every list it names (in parallel) and the platform metadata through the shared client
(`network.md`), each saved to disk with a five-minute lifetime; `saved` rebuilds the same snapshot from disk with no network at all.
A list or the platform metadata that fails does not spoil the rest: a saved copy stands in (the snapshot says so) or the part is
left out, and each such event is a `Problem` with a hint where there is something to do. An app in two lists appears once. Measured
on the real catalog: 4 lists, 232 apps and 232 release records in about 650 ms cold and 30 ms from disk.

`LibraryStore` keeps `apps.json`: a missing file is an empty library, but a file that cannot be read **as a whole** is an error and is
never overwritten (the host keeps the library read-only until the person asks for it to be set aside); `save` writes a temporary file,
flushes and renames it over, copies the file it replaces to `backups/apps-<hash>.json` (the newest 20 are kept) and takes a lock so
two running copies cannot save at once.
