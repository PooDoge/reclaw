# Installing

- last-verified: 2026-10-06
- owner-paths: reclaw-install/**, reclaw-app/src/host/install/**, reclaw-ui/src/settings/location.rs, reclaw-ui/src/catalog_data.rs, reclaw-ui/src/desktop/dialogs/install.rs, reclaw-ui/src/components/install_dialog.rs, reclaw-ui/src/deck/pages/install.rs, reclaw-ui/src/deck/state/install.rs, reclaw-ui/src/deck/app/text_boxes.rs

What pressing Install does, from the form to a folder on disk, and what Update, Uninstall, Verify, Check for updates and Open folder do.
Rules are Quiver's (read from its source, not run) unless the text says otherwise. Decision records: ADR 0017, ADR 0018, ADR 0019 (failure log), ADR 0020 (`filesToAdd`).

## The form

Both interfaces ask the same two things: **Install location** and **Keep pre-release builds** (off). There is no file to choose:
Quiver downloads a prebuilt release and so does Reclaw. (An earlier version asked for "your own game file" and kept Install disabled
until one was picked, and had a "Create desktop shortcut" switch that did nothing; both were inventions and are gone. A shortcut
needs a launch-by-name mode on the command line first.)

The location box starts, **every time the form opens**, from Settings > Library > *Default install location*; when that is empty it
starts from `~/Reclaw/Apps` (`settings::default_install_location`). The Deck's Install page is seeded when it is shown
(`DeckState::take_text_seeds`), and the Deck's Settings text boxes are seeded the same way from what is stored. An empty box at
submit time means the default as well (the host applies it). `~` is the home folder; anything else must be a full path, or the install
is refused with a notice that says so. Each app goes in its own folder inside the location, named by the catalog's `folderName` (one
plain name: a name with a `/` or `..` is refused). The folder is recorded in the library entry's `installPath` (Quiver's field), so
changing the default later does not lose apps installed under the old one. Changing the default also looks again for apps with no
recorded folder (`Host::rescan_installs`), so pointing it at a folder of existing installs, Quiver's for one, shows them as installed
without a restart; an install that is running keeps showing as running.

## Choosing the release and the file (`reclaw-install::{release, policy, matcher, source}`)

* **Which release.** A pinned `preferredVersion` that exists wins. With pre-releases allowed, the newest release that has files. Otherwise
  the tag GitHub calls *latest* if it has files, then the newest stable release with files, and only if there is none the newest
  pre-release. Drafts are never offered. GitHub is asked for `/releases/latest` alone first; the list (30) only when pre-releases or a pin
  need it, or when latest has no files. GitLab: the project's release list. Explicit actions (Install, Update, Check for updates) revalidate
  with the server (a conditional request); a background look may use an answer under five minutes old.
* **Which file.** Metadata, checksums, signatures, symbol files and source archives are never offered (`names::is_auxiliary`). The
  catalog's `releaseAssetFilter` narrows the list. A file fits this machine by Quiver's markers (`Linux-X64`: a `linux`/`appimage`/`tar.gz`
  label and no other system's marker, no other architecture; a bare `.zip` counts as the Windows build). On Linux the Windows builds also
  qualify, after the native ones. Packages for a system installer (`.deb`, `.rpm`, `.flatpak`), phones and macOS installers are
  named in the error, not attempted.
* **Without asking.** Where Quiver asks whenever more than one file qualifies, Reclaw takes the only one, or the only native one when
  Windows builds are also offered. If several native builds remain it takes the best ranked (`.tar.gz`, `.tar.xz`, `.zip`, `.7z`, then
  AppImage, which needs FUSE) and writes the choice and the number of candidates to the log. **A chooser dialog is not built**; use the
  catalog's `releaseAssetFilter` to steer.

## The install (`reclaw-install::install`)

1. The file is downloaded to `<cache>/downloads/<hash of the address>/<name>` by the shared network layer: streamed to disk, hashed,
   resumable, cancellable; checked against the release's SHA-256 (`digest`) and size when the host states them. A mismatch deletes it.
2. Its kind comes from its name; a name with no extension (a GitLab package link) is told by its first bytes (zip, 7z, rar, gzip, xz),
   and a file that is not an archive is a program in one file. A name has an extension only if what follows its last dot is one to five
   letters or digits: `Game-v0.9.2-Linux-X64-Release` has none (ADR 0018). `.elf`, `.x86_64`, `.arm64` and `.aarch64` name a program in
   one file.
3. A first install that has no finished copy writes `install-incomplete.txt` in the folder; an update never does.
4. The archive is unpacked into `<folder>/.reclaw-stage/tree` (same disk as the folder, so the move is a rename). Zip, tar.gz, tar.xz and
   7z are read by the program itself. RAR, and a zip using a method the reader lacks, go to `unrar`, `7z`/`7zz`/`unzip` or `bsdtar` if one
   is on the machine; otherwise the error says what to install.
5. A zip holding only a `.tar.gz` is unwrapped once more. If the tree has no program at the top and is a single folder with no sibling
   files whose inside holds a program, that folder's contents are brought up (repeatedly); anything else keeps its structure.
6. The tree is laid over the app's folder: folders merge, a file replaces a file, **nothing the release does not contain is touched**
   (saves, settings and config files beside the program survive an update). A single-file download is copied in; a new AppImage
   removes the older `.AppImage` files at the top.
7. Programs are made executable (a zip made on Windows has no modes; an ELF file is still a program). Programs are found by what they
   are, not what they are called: `.AppImage`/`.x86_64`/`.arm64`/`.aarch64`, files with an ELF (executable or PIE with an entry point) or
   `#!/` header, and `.sh`; a Windows `.exe` counts only when nothing native is as near the top of the folder as the nearest `.exe`
   (a Windows zip that ships its sources can hold ELF test files deep inside, ADR 0018), and then the install says it needs a runner.
   Wine and Proton prefixes are never searched.
8. No program: the install fails (`NoProgram`) and the folder stays marked incomplete. Otherwise `version.txt` (the release tag) is
   written **last** and the marker removed. The staging folder and the download are deleted; after a failure or a cancel the download
   is kept for the next try.
9. The catalog's `filesToAdd` (empty marker files such as `portable.txt`) are created in the folder if missing, as Quiver does after
   every install; they are never overwritten. When the folder already held an install, the person is told that the game's earlier
   settings stay where it kept them (for the recomps, `~/.config/<Game>`). Mods depend on this (spec: mods, ADR 0020).

If the folder already holds a finished install of exactly the release that would be installed, nothing is downloaded and the user is
told it is up to date. A failed first install leaves the app marked *Failed* until the next try; a failed or cancelled update leaves
the old version installed and working.

## Archive safety (`reclaw-install::archive::sink`)

Every format writes through one sink. No entry may leave the folder (`..`, absolute paths, drive letters; backslashes are separators);
nothing is written *through* a link an earlier entry made (so a link to `..` followed by a file "inside" it fails); a link is made only
if it points inside the folder (others are skipped and logged; the install goes on); set-id bits are dropped, owner read/write is
kept; a file where a folder is needed is an error; at most 500 000 entries and 64 GiB unpacked (a bomb is stopped before the disk
fills). Tools from the system are used only for RAR and rare zips, and their output is walked once for escaping links.

## What is shown

The host reports `ActivityEvent`s from the worker thread: `Started`, `Progress` (Downloading with bytes and rate, Extracting, Finishing)
and `Finished` (an update carries a changelog: from, to, the release's notes and page) or `Failed` (a one-line reason for the row, and
details for the notice: the cause, what to do, and where the log is), preceded by `Log`: the lines the job wrote, from "install
started" (repository, host, platform, filter, pinned version, folder) and "release chosen" (tag, file) to the failure, for the log view
behind the "Failed" label (notices-and-holds, ADR 0019). The library's `GameEntry`s are rebuilt from the library file, the
catalog and an install-state table (`catalog_data::InstallStates`): *Installing* while a job runs, *Failed* after a failed first install,
*Installed* with the folder's version, and *Update ready* when the catalog's latest tag (or a newer one found by Check for updates) is
newer by Quiver's version rules (`reclaw_games::version`: `v1.4.2` is `1.4.2`; `1.4.2-beta` is only itself). What is installed is read
at start from each library entry's folder: `version.txt` present and no marker. Whether a program is really there is checked by
Verify and when playing, not for the whole library at start.

## Other actions

* **Update** is an install into the recorded folder. **Check for updates** asks the host service and says whether a newer release exists.
* **Uninstall** (after the confirmation) deletes the app's folder and clears its `installPath`; the app stays in the library. It refuses
  an address that is not a full path in its simplest form, fewer than four path components, a folder that holds the home folder or one of
  Reclaw's own folders, and a folder that is not marked as an install (`version.txt`, `install-incomplete.txt`, or a program in it). A
  link to a folder is removed without touching the folder.
* **Verify** looks at the folder again and says the version, the program it would start, and whether it needs Wine or Proton. It does not
  compare files with the release (a release lists none).
* **Open folder** opens the recorded or default folder, or says where it looked.
* **Cancel** (on the Downloads row) stops the job; the partial download is kept.

## Not built / not verified

* **Verified against the real services** on 2026-10-06 from the development sandbox (Linux x86-64), with the live tests below:
  * GitLab, end to end through the real API and downloads: Mario Kart 64, Star Fox 64, Duke Nukem: Zero Hour and Extreme-G
    (sonicdcer: package links with no extension, a zip holding a tar.gz), Link's Awakening DX HD (a 7z; install v2.0.7, update to v2.0.8),
    Digimon World (Windows-only zip: installed, reported as needing Wine or Proton). Through the host: install to the default location,
    Verify, Play under a virtual display then Stop, Check for updates (current, then an update offered), Update with a changelog and a
    save beside the program kept, Uninstall.
  * GitHub downloads (`github.com/.../releases/download/...`, redirected to `release-assets.githubusercontent.com`): Zelda64Recomp
    (a zip; v1.2.1 then v1.2.2), OpenRCT2 (an AppImage; v0.5.4 then v0.5.5, the older AppImage removed), doukutsu-rs (one `.elf` program).
  * **Not verified: GitHub's release API** (`api.github.com/repos/.../releases`). The sandbox's proxy refuses it for repositories outside
    the session, so the GitHub cases were described by hand (`RECLAW_LIVE_GITHUB_DIRECT=1`) and the host-level GitHub test fails there
    with the proxy's 403. Run both live suites on a normal network to close this.
  * Play: Extreme-G started and ran until stopped; Mario Kart 64 started and exited with its own "Failed to preload executable!", which
    the same binary also prints when started by hand in the sandbox (an environment limit, not Reclaw's). No game was played with a ROM,
    sound or a GPU. The Wine/Proton path was not run (no runner in the sandbox).
* No chooser dialog; no per-app memory of a chosen file; no GraphQL batching of release lists; no background update passes
  (M3); no desktop shortcut or Steam shortcut; no Flatpak bundles; no Android; macOS and Windows code paths are written and unverified.
* AppImages need FUSE on the machine; the program does not check.
* RAR needs a tool on the machine.
* No free-space check before unpacking: a full disk is reported as the disk being full, and the staging folder is removed.
* Launching and stopping are described in `launch.md`.

## Tests

Live, by hand (download real games; see the verification note above):
`cargo test -p reclaw-install --test live -- --ignored --nocapture --test-threads 1` and
`xvfb-run -a cargo test -p reclaw --lib live -- --ignored --nocapture --test-threads 1`.

`reclaw-install` (119): file-name rules, platform matching, asset choice, release parsing and selection, version rules, every archive
format with real archives, the hostile-archive cases, wrapper hoisting, merging, program discovery, the whole install against a fake
GitHub (zip, tar.gz with links, AppImage, bare program, Windows-only, update over old files, corrupt archive then retry, failed update
keeps the old, hash mismatch, cancel, hostile zip), and uninstall safety. `reclaw-app` host tests drive the same through `Effect`s
(install, empty and `~` locations, the default changing, found-at-start, update with changelog, current-version short circuit, update
check both ways, uninstall, verify, cancel, double press, failures with details, a path for a folder name). `reclaw-ui` tests for the
location rule, the install states, the form pages of the Deck, and that a rebuilt game list keeps what is running.
