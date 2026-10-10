# 0025 quiverlauncher.com is the catalog, and an unverified release waits for a second press

- status: accepted
- date: 2026-10-10
- spec: ../specs/community.md
- supersedes-part-of: 0023 (the "site as enrichment" and "not holding back unverified releases" decisions)

## Context

ADR 0023 read quiverlauncher.com beside the frozen GitHub lists and kept the lists as the browse source. Since then apps keep being
added to the site and never reach the lists, so Reclaw's Catalog stopped growing. Quiver 3.5 browses the site's REST API
(`api.quiverlauncher.com/api/v1`, falling back to the Convex deployment's own host), updates catalog apps only to the release the
site verified, checks each download against the SHA-256 the site recorded, asks before installing an unverified, blocked or
antivirus-flagged release, and reports a missing or mismatching file back to the site. The website Jim looked at syncs over a
Convex websocket (`wss://convex.quiverlauncher.com/api/1.46.0/sync`); the launcher does not use it.

The site still cannot be reached from the sandbox this was built in, so everything below is derived from Quiver's source and its
tests (which hold real answers), and checked against fake servers.

## Decision

* **The site is the catalog.** `CatalogSync` reads the site's listing and release feed in parallel with the lists. The catalog is the
  site's apps first, newest first, then the list apps the site does not have. A list entry that links to a site app adds only what
  the site lacks (its `reclaw` block, its `catalogId` and the platform release). A site app the library holds takes the library's
  repository, source and folder, so a game is one tile. If the lists fail and the site answers, the site alone is the catalog; if the
  site fails, the lists are, with a notice that newer apps are missing.
* **Library apps follow the site** as Quiver's `CatalogLibrarySync` does. Name, project, icon and tags change only where the person
  has not changed that field since the site last set it. The `catalog` snapshot records what the site set.
* **Sorting and filtering happen in Reclaw**, over the whole listing: sort by added, updated, rating, name or system. Filter by
  "runs on this computer", project type and AI use, and hide what is already in the library. These are the same choices and
  defaults as Quiver's Browse, kept in the Catalog section of Settings. Cards show the players' verdict and a NEW badge (added in the
  last 30 days).
* **Catalog apps install the site's choice of release:** the person's pin, else the verified release, else the newest one that is
  not blocked. The release comes with the site's https links and checksums, so the file is checked against what the site saw. A
  rolling release (rebuilt under one tag) is fetched from its repository instead, so its host's current digest is the one checked
  (the site's, when the host states none).
  Update checks for a linked app compare with the verified release and never ask GitHub.
* **A release the site did not verify waits for another press of Install.** Reclaw has no dialog for this. The first press downloads
  nothing and shows a notice: what the site says and why, and "press Install again within 2 minutes". An unverified release, or a
  verified one VirusTotal flags, takes one more press; a blocked one takes two. A file the site did not check, in a release whose
  other files it did, counts as unverified (Quiver's rule).
* **A broken download is reported.** A 404 or 410, or a checksum mismatch on a file the site had a checksum for, is POSTed to
  `/apps/{slug}/download-problem`, as Quiver does. For a missing file, the next press of Install is offered the newest other verified
  release. The notice says so.

## Rejected

* **The website's Convex websocket sync.** It is the website's private client protocol, versioned in the URL (`1.46.0`), and pushes
  state Reclaw does not need live. The REST API is what Quiver itself uses and is stable and cacheable.
* **Server-side sorting and paging, as Quiver does** (`/apps?sort=…&cursor=…`). Reclaw already holds the whole listing (it is the
  catalog), so sorting and filtering locally is instant, works offline from the saved copy, and keeps one source for the Catalog and
  the library's links.
* **Keeping the frozen lists primary** and only adding site apps they lack. The site is where apps are added and renamed now; the
  lists keep only what the site does not have (and the `reclaw` blocks).
* **A confirmation dialog.** Dialogs are built per surface (`surface::presentation`). The Desktop and Deck flows both start installs
  from several places. A second press works the same everywhere and keeps the decision in the host, where the release is known.
  The cost is that a mistaken second press installs. That is why a blocked release needs two more presses, and why the window
  expires.
* **Installing the newest non-blocked release even when a verified one exists** (what Quiver's selection code does, though its comment
  says otherwise). That would make most installs of a catalog app ask for confirmation; preferring the verified release asks only
  when the site has verified nothing yet.
