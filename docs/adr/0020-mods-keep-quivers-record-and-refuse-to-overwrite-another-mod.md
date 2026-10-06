# 0020 Mods keep Quiver's record, refuse to overwrite another mod, and the recomps get their `portable.txt`

- status: accepted
- date: 2026-10-06
- spec: ../specs/mods.md

## Context

Mods were drawn (a Mods page, a detail page, a section on each game) over sample data; Install answered "not built yet". Fifteen
entries of the community catalog have a `mods` block: thirteen Thunderstore communities and six GameBanana games, mostly the N64
recompilations, whose runtime (N64ModernRuntime) loads every `.nrm` file and every folder in a `mods` folder.

Quiver installs mods by unpacking over the mods folder and recording each mod's files in `.quiver-mods.json`. Reading it, and the
recomp runtime, showed three things to decide:

* Quiver overwrites any file in the way, including another mod's, and an update leaves the old version's files that the new one
  dropped. Nothing is staged, so a failure half way leaves a mixed folder.
* A mod packed as `mods/x.nrm` lands in `mods/mods/x.nrm`, which the runtime opens as a mod and rejects.
* The catalog asks for `portable.txt` (`filesToAdd`) beside every recomp. Quiver creates it after each install; Reclaw did not. Without
  it the recomp reads `~/.config/<Game>/mods`, not `<game>/mods`, so every mod Reclaw placed would have been ignored.

## Decision

* **The record is Quiver's file and format**, unchanged, so a folder modded by one launcher is understood by the other, and Quiver
  users moving to Reclaw keep their mods.
* **An install never replaces a file another mod's record lists.** It stops before placing anything and names the mod that owns the
  files. A mod's own files may be replaced by its update.
* **An install is staged and undoable**: unpacked beside the game (not in the mods folder), moved in file by file with whatever it
  replaces set aside, and put back if a move or the record's save fails. Only then are the previous version's leftover files removed,
  with the folders that leaves empty.
* **A wrapper folder named like the mods folder is dropped**, when every file of the archive is in it.
* **An unreadable record stops everything** before a download, and is never written over.
* **`filesToAdd` are created after every game install and update, and before every mod install**, never overwriting. A game installed
  before this is told once, when the file is added, that its earlier settings and saves stay in `~/.config/<Game>`.
* The pure mapping from listings and records to the screens' `ModEntry` lives in the host (`reclaw-app/src/host/mods/entries.rs`), so
  `reclaw-ui` does not depend on `reclaw-mods` or `reclaw-install`.

## Rejected

* **Our own record format** (with enabled flags, hashes, the archive's checksum). It would cost compatibility with Quiver for things
  nothing uses yet. Extra fields could be added later, once it is checked that Quiver's reader keeps fields it does not know.
* **Overwriting like Quiver and recording the new owner.** The first mod's record would then list a file it no longer has, and removing
  either mod would break the other. Refusing is safe and the message says what to remove first.
* **Not writing `portable.txt` into existing installs**, to leave their settings where they are. Mods could then never load for those
  games, which is the whole feature; the notice says where the old settings are.
* **Pointing mods at `~/.config/<Game>/mods` instead.** It works only for the recomps, and only while there is no `portable.txt`,
  which the catalog asks for; it would also make the mods folder depend on a path the catalog does not describe.
* **Managing the recomp's enabled list (`mods.json`).** Its format belongs to the runtime and its menu already does this; writing it
  from outside risks fighting the game. Left for later, if the runtime's format is pinned down.
