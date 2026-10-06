# 0017 Install by laying files over the app's folder, and there is no game-file step

- status: accepted
- date: 2026-10-05
- spec: ../specs/install.md

## Context

The first Install button disabled itself until the user chose "their own game file", copied from an assumption that recompiled games are
built from a ROM. Quiver does no such thing: it downloads a prebuilt release and unpacks it. Pressing Install on any real entry
could therefore never work, and the form also ignored the default location in Settings. The catalog had no installer behind it at all.
Quiver's installer has known weaknesses (an update that fails leaves old and new files mixed; no checksum; only the first page of
releases) that were worth deciding about rather than copying.

## Decision

* **No game-file step.** Removed from both interfaces and the effect vocabulary (`ChooseFile`, `StartInstall.game_file`, `NeedsFile`).
  The same goes for a "desktop shortcut" switch that did nothing; it comes back with a launch-by-name command line.
* **Files are laid over the app's folder, not swapped in.** An archive is unpacked into a hidden staging folder inside the app's
  folder (same disk), a lone wrapper folder is brought up, and the tree is moved over what is there: folders merge, files replace
  files, nothing the release lacks is deleted. `version.txt` is written last. A first install is marked `install-incomplete.txt` until it
  is done; a failed update leaves the old version installed, because nothing is touched until the whole archive has unpacked cleanly.
* **The library entry's `installPath` records where an app went** (Quiver's own field), so `apps.json` stays Quiver's format and no second
  file can drift from the folders. What is installed is read from the folder itself (`version.txt`, no marker).
* **Unpacking goes through one sink** that enforces path safety, link rules, mode rules and size limits for every format, and zip, tar.gz,
  tar.xz and 7z are read by the program itself.
* **One native build is installed without asking.** The release's SHA-256, when GitHub states one, is checked.

## Rejected

* **Swapping the whole folder (rename the old away, rename the new in).** Many recompiled games keep saves, config and mods beside
  the program; a swap deletes them unless they are copied across by rules that need to know each game. Overlaying keeps them, and the
  staging step already gives the main benefit of a swap, that a bad archive changes nothing.
* **A separate `installs.json` with the folders and versions.** A second record can disagree with the disk; the library entry already has
  the field, and the folder says its own version.
* **Shelling out to `unzip`, `tar` and `7z` for everything.** Behaviour then depends on what the machine has (Bazzite is immutable and
  has few), the path-safety rules would be the tools' and not ours, and progress and cancel would be lost. Tools are used only for RAR
  and the rare zip method.
* **Asking whenever more than one file qualifies, as Quiver does.** A Windows zip beside a Linux build is the normal case, and a
  pad-driven launcher should not ask about it. A chooser is still wanted for the truly ambiguous case, and is not built.
* **Checking free disk space before unpacking.** The figure for an archive's unpacked size is not known for tar streams, and a wrong
  refusal is worse than a clear "the disk is full" after the fact, with the staging folder cleaned up.

## Consequences

An update can leave files from an older release that the new one no longer ships. Quiver does too. The first live installs have not been
done from the build environment (the proxy refuses the download hosts), so the real-world mix of archive layouts is untested.
