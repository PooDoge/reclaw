# Community: what quiverlauncher.com says about a game

- last-verified: 2026-10-10
- owner-paths: reclaw-catalog/src/site/**, reclaw-catalog/src/snapshot.rs, reclaw-sync/src/site.rs, reclaw-sync/src/site/**, reclaw-app/src/host/community.rs, reclaw-app/src/host/community/**, reclaw-ui/src/community/**, reclaw-ui/src/desktop/pages/game/community.rs

Since Quiver 3.5 (2026-10-09) the community catalog lives on quiverlauncher.com, and the GitHub lists Reclaw browses are frozen.
Reclaw keeps browsing the lists and reads the site beside them, for what the lists never had: how a game runs for the players who
tried it, their reviews, who made it, whether AI wrote it, and what the site checked about each release. Decision record: ADR 0023.
The API shapes and the wording come from Quiver 3.5's source (`QuiverWebsiteClient`, `BrowseText`, `BrowseDetailsViewModel`,
`ReleaseWarnings`), read, not run: **the site could not be reached from the sandbox this was built in**, so it is checked against a
fake server only.

## Where it is read from

* `https://quiverlauncher.com/api/v1`, and `https://quiver-launcher.vercel.app/api/v1` when the first cannot be reached (a DNS,
  connect, TLS or timeout failure, or status 499). Once the fallback has answered it is used for the rest of the run. Quiver does the
  same. `QUIVER_API` replaces both with one address (tests, a local copy of the site).
* Every list is paged `{items, nextCursor, isDone}`, 100 a page; `isDone` wins over a cursor, and more than 50 pages is an error
  rather than a loop. An item that does not read is skipped and counted, not fatal; unknown enum values read as the careful answer
  (an unknown release state is *unverified*, an unknown scan verdict *pending*).
* Through `reclaw-net`'s cache with stale-on-error: the listing (`/apps`, `/release-status`) for 30 minutes, a game's page
  (`/apps/{slug}`, `/reviews`, `/release-history`) for 2. A 404 for a page means the site no longer lists the game: no page,
  no releases, not an error.

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

## The library keys it keeps

`catalogEntryId` and `catalog` (Quiver's snapshot of the site's name, project, icon and tags) are read and written back unchanged
(spec: catalog-format). Reclaw does not yet move a field when the site changes it.

## Not done

Browsing the catalog from the site; installing only verified releases (with the site's checksums) and the *back to verified* reinstall;
the warning before installing a flagged release; reporting a failed download to the site; ratings on catalog cards; the original
game's page. Quiver's anonymous usage counts are deliberately not sent.
