# Mods

- last-verified: 2026-10-10
- owner-paths: reclaw-mods/**, reclaw-app/src/host/mods/**, reclaw-ui/src/desktop/pages/mods/**, reclaw-ui/src/mod_shelves.rs, reclaw-ui/src/mod_games.rs, reclaw-ui/src/desktop/pages/game/mods.rs, reclaw-ui/src/desktop/pages/mod_detail.rs, reclaw-ui/src/desktop/pages/common/mod_row.rs, reclaw-ui/src/store/changes.rs

Which games take mods, where their mods are listed, and what Install, Update and Remove do to a game's folder. The format of the
record and the site addresses are Quiver's (read from its source, not run) unless the text says otherwise. Decision records: ADR 0020, 0022, 0024.

## Which games, and where their mods come from

A game takes mods when it is in the library, is installed (or is a manual app whose folder exists), and has a usable `mods` block:
a `path` (a folder inside the game's folder, `/`-separated, no `.` or `..`) and at least one source Reclaw can read. The library's
block is used when it is usable, otherwise the catalog's (a library written by an older program may lack it).

A source is `{provider, sourceUrl}` (`reclaw-mods::source`):

* `thunderstore`: `https://thunderstore.io/c/<community>/...`, or a bare community slug.
* `gamebanana`: `https://gamebanana.com/games/<n>`, `https://gamebanana.com/mods/games/<n>`, or a bare number.

Anything else is skipped (logged at debug), so a catalog can name a provider a newer program knows.

## Listing (`reclaw-mods::client`, `reclaw-app/src/host/mods/list.rs`)

The sites are asked after every catalog refresh, after a game install finishes, and when the person presses **Refresh** on the Mods
page. Each source is asked in four orders (`list.rs`, `ORDERS`): most downloaded (up to 60 mods), top rated, last updated and newest
(up to 20 each). The orders are merged into one list (`entries::merge`): a mod once, at its place in the first order that listed it,
with its place in every order that did (`ModRanks`). Most downloaded is the source's answer: when it fails the source has failed (as
below); when another order fails, its shelf is missing and the failure is logged at warn.

* Thunderstore: `ordering=most-downloaded`, `top-rated`, `last-updated`, `newest`. GameBanana: `_sSort=Generic_MostDownloaded`,
  `Generic_MostLiked`, `Generic_NewAndUpdated`, `Generic_Newest`.
* Dates: Thunderstore's listing `datetime_created` / `last_updated` (the package endpoint's `date_created` / `date_updated`),
  GameBanana's `_tsDateAdded` and `_tsDateUpdated` (or `_tsDateModified`), read as Unix seconds or RFC 3339. Rating: Thunderstore's
  `rating_count`, GameBanana's `_nLikeCount`. **These names and the order values were taken from the sites' documentation and
  Quiver's source, not seen in a live answer**; a missing date is shown as no date, and a missing order as no shelf.

The addresses:

* Thunderstore: `/api/cyberstorm/listing/<community>/?page=<n>&ordering=<order>&nsfw=false&deprecated=false`, restricted
  to the community's *Mods* section when `/api/cyberstorm/community/<community>/filters/` names one (so modpacks and tools stay out).
  The listing gives no version; the icon's name (`<Owner>-<Name>-<version>.png`) does.
* GameBanana: `/apiv13/Mod/Index?_nPerpage=50&_aFilters[Generic_Game]=<n>&_nPage=<n>&_sSort=<order>`. Mods that cost money are left out.

Answers are cached on disk: a listing for 15 minutes, the Thunderstore section for 12 hours; an old copy is used when the site does not
answer. Refresh asks again regardless. A source that fails keeps the game's previous listing; the failure is logged, and shown as a
notice only when the person pressed Refresh (a launch without a network would otherwise open on a stack of notices).

What the screens get (`entries.rs`): the installed mods first, in the record's order, then the rest of the listing in the site's order.
Deprecated and adult mods are shown only when installed. An installed mod the site no longer lists is still shown, so it can be removed.
`_` in a name is shown as a space. A mod is **Update** when the site's version is newer than the installed one: versions that read as
numbers compare as release tags do; two that do not (GameBanana allows any text) are an update when they differ.

## The Mods tab and a game's page (`reclaw-ui/src/mod_games.rs`, `mod_shelves.rs`, `desktop/pages/mods/`, `desktop/pages/game/mods.rs`)

The host sends the games that take mods (above) as their own list, `AppAction::SetModdable`, every time it sends the mods. The
screens do not work it out from the mods: a game whose sites have listed nothing yet, or did not answer, still takes mods.

* **The game chip** on the Mods tab ("All games", or the chosen game's name) opens a list of "All games" and every game that takes
  mods, by title, each with how many mods are listed for it. Choosing one narrows the list to its mods; the site chips' counts and the
  heading ("2 mods for Starfall 64") count within it. The choice stays while moving between pages, like the site chip and the search.
  A chosen game that stops taking mods (uninstalled) means all games again. Choosing a game clears the Mods search and closes an
  opened shelf.
* **Shelves** (`mod_shelves.rs`), when nothing is searched: *Installed*, *Most downloaded*, *Top rated*, *Recently updated* and
  *New releases*, in that order, each holding the mods (within the game and site chips) that its site order listed, in that order
  (`ModRanks`; with two sites, places are interleaved by rank then downloads). An empty shelf is left out. Each shows its first 4 mods
  (2 columns on a wide window, 1 below) and **Show all N**, which opens the shelf on its own: 20 mods a page with Previous / Next and
  "Page 2 of 3 · 21–40 of 57", and **All shelves** to go back. The opened shelf is frame state (`DesktopUi::mod_shelf`), like the chips.
* **A search** on the Mods tab (see `search.md`) replaces the shelves with the matching mods: every word in the title, author, summary
  or tags; title matches first, then most downloaded; 20 a page. It searches the mods already listed, not the sites.
* **A row** says, after the title: author · site · version · downloads · ratings or likes · "updated 3 days ago" when the site gave a date.
* **A game's page**, for a game that takes mods, has **Browse mods** under Links and a *Mods for this game* section even before
  anything is listed. Both open the Mods tab on that game with the site chip on All sites and the search cleared, so it shows
  everything listed for the game. A game that does not take mods has neither.

## Install and update (`reclaw-mods::install`, `reclaw-app/src/host/mods/jobs.rs`)

One job per mod, on its own thread, shown on the activity board like a game's install and cancellable from it. In order:

1. The catalog's `filesToAdd` are created in the game's folder if missing (`portable.txt` for the recomps; see *Install* below).
2. Thunderstore dependencies (`Owner-Name-1.2.3`) are installed first, newest version, each once, up to 16 deep. One already recorded
   for the game is skipped, whatever its version. A dependency the site does not have fails the whole install before anything
   of the mod is placed.
3. The game's record (`.quiver-mods.json`) is read **before anything is downloaded**. One that cannot be read stops the install and is
   left exactly as it is.
4. The file is downloaded through the shared network layer (resumable, cancellable) to `<downloads>/mods/<hash of the address>/`.
   Thunderstore: the newest version from `/api/experimental/package/<owner>/<name>/`. GameBanana: the mod's first archive among its
   files that are not archived, otherwise its first file.
5. An archive (by its name, or by its first bytes when the name has no extension) is unpacked into `<game>/.reclaw-mod-stage/` with the
   same safety rules and limits as a game (no paths out, no links out, 16 GB, 200 000 entries). Any other file (a `.nrm`, which is a
   zip inside but is the mod itself) is placed as it is.
6. Where each file goes (`plan.rs`):
   * Thunderstore's description files at the top of the archive (`manifest.json`, `icon.png`, `README.md`, `CHANGELOG.md`) are left out.
   * An archive whose every file is inside one folder named like the mods folder (`mods/x.nrm`) was packed relative to the game: that
     folder is dropped. **Quiver does not do this**; without it the recomp runtime would find a folder `mods/mods`, open it as one mod
     and fail.
   * With `layout: folderPerMod`, an archive with any loose file at its top goes into a folder named after the mod.
7. A file another mod's record lists is a **conflict**: the install stops before placing anything and says which mod owns which
   files. The same mod's own files (an update) may be replaced. **Quiver overwrites them.**
8. Files are moved in one by one; a file already there is set aside first. If any move or the record's save fails, everything placed is
   removed and everything set aside is put back.
9. The record is written (to a `.tmp` file, then renamed), then the files the previous version had and this one does not are removed,
   with the folders that leaves empty. The stage and the download are deleted.

The record is Quiver's: `{"mods": [{provider, sourceKey, id, fullName, owner, name, version, downloadFileId, downloadFileName,
files}]}`, camelCase, `files` relative to the mods folder. A folder modded by either launcher can be read by the other.

## Remove

The files the record lists for the mod are deleted, then the folders that leaves empty (the recomp runtime treats every folder in
`mods` as a mod, and an empty one as an error), then the record. Files nobody recorded (the person's own) stay. A recorded path that
is outside the mods folder, or is now a folder, is not touched. A file that cannot be deleted stays listed in the record, so Remove can
be pressed again. Uninstalling the game removes its mods with its folder.

## Install: `filesToAdd`

The catalog's `filesToAdd` (empty marker files) are now created after every game install and update, as Quiver does
(`reclaw-install::layout::add_marker_files`, never overwriting). For the recomps this is `portable.txt`, which makes the game keep its
settings, saves **and the list of enabled mods** in its own folder and read mods from `<game>/mods`. Without it the runtime reads
`~/.config/<Game>/mods`, so mods placed in the game's folder would never load. Before this change Reclaw did not write the file; for a
game installed then, the first install or update (of the game or of a mod) adds it and says that earlier settings and saves are still
in `~/.config/<Game>` and can be copied over.

## Not built / not verified

* **Not verified against the real sites.** The development sandbox's proxy refuses thunderstore.io and gamebanana.com, so every
  address and answer shape above was taken from Quiver's source and its tests and checked only against a fake server. Run the live
  suite on a normal network: `cargo test -p reclaw-mods --test live -- --ignored --nocapture --test-threads 1`.
* Not run with a real game: that a recomp loads a mod placed this way, and that `portable.txt` moves its settings as described (read
  from Zelda64Recomp and N64ModernRuntime's source).
* No enable / disable. A recomp enables mods in its own `mods.json` in the game's folder; the game's own menu does it.
* GameBanana mods with several files: the first archive is taken; there is no chooser.
* Only the first 60 mods by downloads and 20 in each other order are listed per source; the search does not ask the sites, so a mod
  outside those is not found. No paging against the sites.
* No Mods page on the Deck.
* Removing a mod does not remove the dependencies installed for it.

## Tests

`reclaw-mods` (32): addresses and answers of both sites, source parsing, the record's load and save, placements and conflicts,
paths that stay inside the folder, and the whole install against a fake site with real zips (dependencies first, an update that drops
old files, a conflict refused, removal with folders pruned, an unreadable record, a single-file mod, a missing dependency).
`reclaw-ui`: `mod_games` (the choices, their order and counts, a choice that is gone), `mod_shelves` (which mods each shelf holds,
the search's order, paging), `mod_row` (the meta line, "ago"), the store's `SetModdable`, and the
desktop pages (the game chip narrows the list; a game's page links to its mods, and only for a game that takes them).
`reclaw` (`host::mods`): the list joined from a listing and a record, the games sent with it, and listing, installing, updating and removing through the host
with `portable.txt` created, a conflict reported as a failed job, and an unknown game refused. Live, by hand: the command above.
