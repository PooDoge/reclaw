# 0023 Read quiverlauncher.com beside the frozen lists, and keep Quiver's link to it in the library

- status: accepted
- date: 2026-10-10
- spec: ../specs/community.md

## Context

Quiver 3.5 (2026-10-09) moved its community catalog to quiverlauncher.com: the app browses the site's API, writes `catalogEntryId`
and a `catalog` snapshot into each library entry, installs only releases the site verified, and shows ratings, reviews, developers,
AI use and VirusTotal results. The GitHub lists Reclaw reads are frozen, kept "so existing installs keep working". Reclaw dropped the
two new library keys on every save, which unlinks a library Quiver shares. The site's API could not be reached from the sandbox where
this was built, so its shapes are known only from Quiver's source.

## Decision

* **The library keeps `catalogEntryId` and `catalog`** unchanged, in Quiver's place in the key order.
* **The site is read as enrichment.** The frozen lists stay the browse source; after a catalog refresh the host reads the site's
  listing, links each game (entry id, then repository, then folder), and puts the result in the store. A game page reads its own
  page, reviews and releases when it opens. The wording is ported from Quiver so both launchers say the same thing.
* **One client (`reclaw_sync::SiteClient`)** on `reclaw-net`, with Quiver's fallback host, paging and leniency, and the data in
  `AppState::community` behind its own channel.

## Rejected

* **Switching the browse source to the site now.** It is the direction Quiver took, but it changes what the Catalog lists and how an
  app is added, and none of it could be checked against the real API. Reading the site beside the lists is useful today and fails
  quietly; the switch can follow once the API has been seen.
* **Fetching the pages from the component with `use_remote_file`.** That path is for pictures and documents under a size and address
  policy; it would bypass the fallback host, the paging and the linking, and each window would read its own copy.
* **Keeping the site's data out of the store** (component state). The Desktop and Deck pages, and the linking done after a refresh
  on a background thread, must agree on it (rule 11).
* **Holding back unverified releases** as Quiver does. It needs the site's per-release checksums wired into install and a way back
  (*Reinstall verified*); until then the page says honestly that Reclaw installs the latest release.
