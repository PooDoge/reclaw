# Community: what quiverlauncher.com says about a game

- last-verified: 2026-10-10
- owner-paths: reclaw-catalog/src/site/**, reclaw-sync/src/catalog.rs, reclaw-sync/src/snapshot.rs, reclaw-sync/src/snapshot/**, reclaw-app/src/host/install/verified.rs, reclaw-app/src/host/install/verified/**, reclaw-app/src/host/install/site.rs, reclaw-ui/src/catalog_browse.rs, reclaw-ui/src/catalog_browse/**, reclaw-games/src/listing.rs, reclaw-catalog/src/snapshot.rs, reclaw-sync/src/site.rs, reclaw-sync/src/site/**, reclaw-app/src/host/community.rs, reclaw-app/src/host/community/**, reclaw-ui/src/community/**, reclaw-ui/src/desktop/pages/game/community.rs

Since Quiver 3.5 (2026-10-09) the community catalog lives on quiverlauncher.com, and the GitHub lists are frozen. The site is
Reclaw's catalog (ADR 0025, which replaces part of ADR 0023). Its apps come first, newest first, and the lists add only what the site
lacks. The site also says how a game runs for the players who tried it, their reviews, who made it, whether AI wrote it, and what it
checked about each release. Catalog apps install the release the site verified, checked against its checksums.
The API shapes and the wording come from Quiver 3.5's source (`QuiverCatalogClient`, `CatalogReleases`, `CatalogLibrarySync`,
`BrowseViewModel`, `GameDownloadInstallService`) and its tests of real answers. They were read, not run: **the site could not be
reached from the sandbox this was built in**, so they are checked against fake servers only.

## Where it is read from

* `https://api.quiverlauncher.com/api/v1`. When that cannot be reached, Reclaw uses the Convex deployment's own host,
  `https://famous-wildebeest-660.convex.site/api/v1`. "Cannot be reached" means a DNS, connect, TLS or timeout failure, a stall, or
  status 499. Quiver does not fall back on a timeout; Reclaw does, because a timeout from a blocked host looks the same. A saved copy
  that stands in for an unreachable host still gives way to a fresh answer from the fallback. Once the fallback has answered, it is
  used for the rest of the run. `QUIVER_API` replaces both with one
  address (tests, a local copy of the site). The website's websocket (`wss://convex.quiverlauncher.com/api/<version>/sync`) is its
  own client protocol and is not used.
* Every list is paged `{items, nextCursor, isDone}`, 100 a page. `isDone` wins over a cursor, and more than 50 pages is an error
  rather than a loop. An item that does not read is skipped and counted, not fatal. Unknown enum values read as the careful answer:
  an unknown release state is *unverified*, an unknown scan verdict *pending*. A field of the wrong type reads as empty: the live
  `"withdrawn": false` is read as no withdrawn versions.
* Through `reclaw-net`'s cache with stale-on-error: the listing (`/apps`, `/release-status`) for 30 minutes, a game's page
  (`/apps/{slug}`, `/reviews`, `/release-history`) for 2. A 404 for a page means the site no longer lists the game: no page,
  no releases, not an error.
* The listing and the feed are read with the catalog, in parallel with the lists (`CatalogSync::with_site`). A site that cannot be
  read leaves the lists, with a notice that newer apps may be missing. Lists that cannot be read leave the site as the catalog.

## The catalog

* **Order.** The site's apps, newest added first, each as a catalog entry built from its listing and its release feed line
  (`site::to_entry`). The entry carries the repository, source, folder, icon, tags, `filesToAdd`, mods and `catalogEntryId`. An app
  with no feed line, slug or name is left out. Then come the list apps the site does not have, in list order.
* **Merging.** A list entry that links to a site app (see below), or has the same repository and folder, adds only its `reclaw`
  block, its `catalogId` and the platform release. Its list's name is added to the app's lists after `quiverlauncher.com`.
* **The library's copy wins its identity.** A site app the library holds takes the library's repository, source and folder, so the
  tile and the card are one game.
* **Library apps follow the site** (`site::follow`, Quiver's `CatalogLibrarySync`). The name, project, icon and tags take the site's
  current value unless the person changed that field since the site last set it. Tags the site added are added, and tags it dropped
  are dropped. The `catalog` snapshot records what was set. The library is saved once if anything moved.
* **Browsing** (desktop Catalog, Settings > Catalog). Sort: recently added (default), recently updated, top rated (Quiver's player-feedback score, (runs well + half
  of has issues + 1) / (everyone + 2), then the number of reviews), name, system. Filters: runs on this computer (default on), project type, AI use, and hide apps
  already in the library (default on, with a line saying how many were hidden). This is all done locally over the whole listing. A
  list-only app has no facts: it sorts last and passes every filter but a project type. Cards show the players' verdict ("Runs well ·
  4 players", coloured by tone) and a NEW badge for an app added in the last 30 days. The Deck's catalog keeps the host's order.

## Which entry is a game's

After every catalog refresh the host reads `/release-status` and `/apps` and links each game in the catalog and the library, first
match wins (`site::link`):

1. the library's `catalogEntryId` (written by Quiver 3.5 when the app was added from the site);
2. the provider and repository, with the release asset filter and then the folder breaking a tie;
3. the folder alone, for a repository that moved.

The library's copy of a game is linked after the catalog's, so an entry id Quiver wrote wins. A game with no entry shows nothing new.

## What is shown

* **Desktop game page.** *How it runs*: the verdict (what most players said, a tie going to the more careful answer: runs well,
  has issues, doesn't run), the counts, up to 10 reviews (who, when, platform, version tested, the result, what they wrote), and
  *Share how it runs*, which opens the site's review tab (reviews are written there; Reclaw has no account). *Releases*: what each
  of the last five is on the site (verified, unverified, blocked), why, a VirusTotal line when a file is flagged or a scan is
  pending, when a waiting release should be verified, a release being checked now, and withdrawn versions. It replaces the catalog's
  *Recent updates* once read. Beside the game, *On quiverlauncher.com*: made by, platforms, the latest release and how old it is,
  the verified version, AI use, and *Open its page*.
* **Deck game page.** The verdict line and the verified version under the badges.
* A page is read when it opens (one read at a time per game) and each part fails on its own: the page says which part could not be
  read and why. Nothing here is a notice; failures are logged (`warn`, with the slug and the part).

## Installing a catalog app

* **The release.** The person's pin, if the site lists it. Otherwise the release the feed calls verified (a pre-release too), or the
  newest verified one with files. Otherwise the newest
  release the site has not blocked, stable before pre-release (`install::verified::target`). Its files are the site's https links
  with the site's SHA-256, so the download is checked against what the site saw. A pin the site does not list, or a site that cannot
  be read, is left to the repository. A rolling release is fetched from the repository, whose current digest is checked.
* **The check.** It is made per file. A release whose other files the site checked, but not this one, is unverified. VirusTotal's
  verdict is the file's own when the site gives one per file.
* **What "verified" does not cover**, as in Quiver: a verified release whose files the site recorded no checksum for installs without one.
  A rolling release is checked against its host's digest, or the site's when the host gives none. A press confirms one file at one
  address with one checksum: if the site changes any of them before the next press, the count starts over.
* **Confirming.** An unverified release, or a verified one that several engines flag, installs only when Install is pressed again
  within 2 minutes. A blocked one needs two more presses. A site that could not be read makes the release unverified ("Reclaw could
  not reach quiverlauncher.com"), unless it is the verified version in the feed. The first press downloads nothing; its notice gives
  the site's reasons and how many presses are left.
* **A broken download.** A 404 or 410, or a mismatch with the site's checksum, is POSTed to `/apps/{slug}/download-problem` as
  `{version, file, problem: "missing" | "mismatch"}`. A failed report is only logged. For a missing file, the failure says which
  other verified release the next press of Install will take, and that press takes it.
* **Updates.** A linked app without a pin is up to date when its installed version is not older than the verified one. No request is
  made. An app the site has not verified anything for is checked against its repository as before.

## The library keys it keeps

`catalogEntryId` and `catalog` are written back in Quiver's place in the key order (spec: catalog-format). `catalog` is Quiver's
snapshot of the site's name, project, icon and tags.

## Not done

The original game's page (`/games/{slug}`); Quiver's "my app list" (a personal subset); kiosk mode; filters on the Deck's catalog;
choosing an older release from the page (Quiver's *Change version*). Quiver's anonymous usage counts are deliberately not sent.
