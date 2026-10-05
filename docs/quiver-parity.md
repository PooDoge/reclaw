# Where Reclaw stands against the Quiver launcher

Quiver (C#, Avalonia) is the launcher whose catalog, installer, library and update behaviour Reclaw is bringing to Rust before improving on
them. This file is the map from its features to ours. It is a plan and a status board, not a spec: what is *built* is in `specs/`.
Everything about Quiver here comes from reading its source and tests (the `quiver-launcher` checkout read on 2026-10-04); **nothing was run**, so each rule is a reading
of the code, not an observation of the program. Where the reading was uncertain the research notes say so, and so do the rows below.

Status: **done** = pure logic with tests; **ui** = drawn over sample data, not connected; **no** = not started.

## Findings that change the plan

1. **Quiver does not build games from ROMs.** An install downloads a prebuilt release asset (a zip, 7z, rar, tar.gz, AppImage, or a single
   executable) from GitHub or GitLab and unpacks it over the app's folder. `filesToAdd` are *empty marker files* it creates beside the
   game (`portable.txt`), not files the user supplies. The user's game file is the app's own business on first launch. Reclaw's install dialog
   has a "choose your game file" step; that is our invention. Decide whether to keep it as an optional convenience or drop it.
2. **Installs are overlays.** No staging directory, no rollback, no checksum. A failed *update* leaves old and new files mixed;
   only a first install is guarded (`install-incomplete.txt`). `version.txt` is written last. We can do better; the plan is to stage and swap.
3. **Only the first page of releases is read**, and a pinned version outside it silently becomes "latest". Drafts are not filtered.
4. **Merge and Replace in the catalog review drop the user's `linux*` fields** (read from the code, not run). Worth deciding on deliberately.
5. **Mods have no disable, no backup, no conflict check and no rollback**, and the provider API shapes could not be checked from the
   sandbox (the proxy refused thunderstore.io and gamebanana.com). Those must be verified against the live services before they are built.
6. **The network layer is `reclaw-net`** (ADR 0010, 0011): one client for the catalog, release APIs, downloads and mod providers, with the
   address policy kept for pictures and READMEs written by strangers.

## Status by area

| Area | Quiver | Reclaw | Next |
|---|---|---|---|
| Library file `apps.json` (read, write, identity, dedupe, backups) | `AppCatalogService`, `LibraryStore` | **done**: strict read, atomic write, lock, hashed backups, fail-closed (`reclaw-sync`) | |
| Catalog lists, community index, platform index | `Catalog*`, `CommunityCatalog*`, `PublishedPlatform*` | **done** parse and validate; checked on the real catalog | M1 |
| Normalisation (tags, files, mods, display name) | `TagHelper`, `AppFilesToAddService`, `GameModsConfig`, `AppDisplayName` | **done** | |
| Systems from tags | (tags only) | **done** 22 systems, specificity rules | |
| Catalog compare and review (4-pass match, New / Changed / Local only, Add / Merge / Replace, ignore, hide, bulk, pending counts, auto-acknowledge) | `CatalogCompareService`, `CatalogSyncViewModel` | **no** | M1 |
| Catalog sources (subscribe, refresh, cache, ETag) | `AppCatalogService`, `CommunityCatalogBootstrap` | **done** for the community catalog (`reclaw-sync`); user-added lists **no** | M1 |
| Platform eligibility (classify a release's assets as Windows / Linux / Mac / Android) | `PlatformAssetMatcher`, `CatalogPlatformSupport` | **no** | M1 |
| Library UI: grid / list / compact, search, sort, tag filters, context menu | `Views/Library*` | **done** over the real library; Add / Remove from the Manage menu | name styles, tag filters, per-user tags |
| Catalog UI | `Views/Catalog*` | **done** over the real 232 apps with system chips, search, status line and Refresh; the review flow (New / Changed) is **no** | review (M1) |
| Release fetching GitHub / GitLab, rate limits, token, request coordinator | `ReleaseRequestCoordinator`, `GitHubApiCache` | the client under it is **done** (`reclaw-net`: retries, `Retry-After`, ETag, per-host limits, tokens); the release lists and their paging **no** | M2 |
| Choosing the asset for this OS and CPU | `DownloadAssetPolicy`, `PlatformAssetMatcher` | **no** (and Linux offers Windows builds beside native ones) | M2 |
| Download, extract (zip 7z rar tar), flatten one wrapper folder, find the executable, `chmod +x` | `GameInstallationService`, `GameDownloadService` | download **done** (streamed, hashed, resumable, cancellable); extraction and the rest **no** | M2 |
| Status machine and version comparison (`ReleaseVersionIdentity`) | `GameStatusService`, `GameInfo` | **no**; Reclaw has `AppStatus` for the UI only | M2 |
| Launch (native, Wine / Proton, AppImage env, Flatpak), process tracking | `GameLaunchService`, `WindowsRunnerService` | partial: `reclaw-runtime` starts and stops process groups; no runner choice | M2 |
| Uninstall to trash, Locate install, Open folder, Force update, change version | `LibraryActions` | **ui** menu entries only | M2 |
| Update checks (cache layers 6 h / 24 h, 60 s deadline, background timer, auto-update) | `LibraryUpdateChecker` | **no**; the activity board shows events the host sends | M3 |
| Mods (Thunderstore, GameBanana, install / update / uninstall, `.quiver-mods.json`) | `Services/Mods/*` | config schema **done**; providers **no**; page **ui** | M4 |
| Settings (General, Controls, Appearance, App cards, Advanced) | `settings.json`, PascalCase | different file and keys (TOML); Reclaw's own settings **done**; Quiver's import **no** | M5 |
| Gamepad and keyboard navigation | SDL2 polling | `reclaw-input` on `gilrs` (not hardware-verified) | |
| Window, tray, close-to-tray, placement | Avalonia | custom frame, monitors, saved placement **done**; tray **no** | |
| Announcements, token banner, launcher self-update (Velopack) | `MainView` | **no** | M3 |
| CLI (`--run`, list, update) | `CLIHandler` | **no** | later |
| Android head | `QuiverLauncher.Android` | not planned | |

## Milestones

* **M1 Catalog**: the library on disk (atomic write, lock, backups, never overwrite on a read error), catalog sources and refresh through the
  fetch client, compare and review, platform classification. Done when the rules in `specs/catalog-format.md` pass against Quiver's own test
  expectations ported as tests, and the Library and Catalog pages show the real 232 apps.
* **M2 Install and launch**: release fetching, asset choice, download, safe extraction, executable discovery, status, launch, uninstall.
  The first end-to-end install of a real project is the milestone's test.
* **M3 Updates**: check passes, notifications, auto-update, self-update.
* **M4 Mods**: after the provider APIs are checked against the live services.
* **M5 Settings and polish**, then the improvements in `docs/ideas.md`.
