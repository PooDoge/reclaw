# Ideas for after parity

Not a spec and not a plan: a menu, written 2026-10-04 so the choice can be made with the costs in view. Nothing here is built. The
facts it leans on are from the community catalog as it stands (232 apps, 4 lists) and from reading the Quiver launcher's source; a claim
about an outside service that I could not check is marked **unverified**.

## Three things to settle before choosing

1. **Parity of capability, not of defects.** Quiver's installer overlays new files on old with no staging, no rollback and no checksum;
   it reads only the first page of a repository's releases; its catalog Merge and Replace appear to drop the user's `linux*` fields.
   Porting those faithfully would make Reclaw "the same" and worse. Each is cheaper to do right while the installer is being written than
   to retrofit. `docs/quiver-parity.md` lists them.
2. **The catalog has no licence.** I found no LICENSE file in `quiver-community-app-catalog`. Reading it from its published address at run
   time is what Quiver itself does; bundling a copy into Reclaw or republishing it is another matter. This is why `reclaw-catalog` reads
   their format and its tests read a checkout through an environment variable instead of copying data in. Ask the maintainer before bundling.
3. **Mods are a data gap before they are a feature gap.** Only 15 of the 232 apps have any mod source (12 Thunderstore, 7 GameBanana).
   A mod browser over the existing catalog would be empty for 93% of games. Whatever we build for mods needs us to curate mod sources, which
   is also the best reason to have a catalog of our own.

## Keeping the collection current ("it just updates")

**1. Staged installs with one-click rollback.** Download to a staging folder, verify, swap, keep the previous version. Bazzite is Btrfs, where
a copy of a game folder can share its data blocks (reflink), so keeping the last two versions costs almost nothing there.
*Pros:* no half-updated games; "this update broke my mods" becomes one click; fixes Quiver's weakest point. *Cons:* more disk on filesystems
without reflinks; saves that live inside the install folder must be carried across; more states to test. *Effort:* M. *Needs:* the installer.

**2. Save guard.** Before an update, copy the game's save data aside, and restore on demand. *Pros:* the thing players fear most about
updates. *Cons:* needs to know where each game keeps saves (next to the executable for some, under XDG paths for others) which means a
`reclaw.saves` field and someone to fill it in; wrong guesses are worse than nothing. *Effort:* M, plus catalog work.

**3. Release channels and a changelog you read in the app.** Stable, pre-release and nightly per game, with the release notes shown through
the existing README pipeline when an update is waiting, and a "What's new" shelf across the library. *Pros:* makes updates feel like
Steam's patch notes; costs little because the markdown path exists. *Cons:* release notes are uneven; nightly tags are not versions and Quiver's
comparison already struggles with them; more API calls against GitHub's rate limit. *Effort:* S to M.

**4. Trust and change alerts.** An app is identified by `owner/repository`, so a transferred or renamed repository, or a new maintainer, would
silently change who supplies the executable. Show a badge for catalog-verified entries, and warn when the repository's owner, its archive
status or the shape of its release assets changes. If GitHub's release assets carry a SHA-256 digest (**unverified** here), check it.
*Pros:* a launcher that downloads and runs programs from a hundred strangers should say who they are; a differentiator. *Cons:* alerts that
cry wolf get ignored; needs a stored baseline per app; verification is only as good as what the hosts publish. *Effort:* M.

## Mods (graphics, features, gameplay)

**5. Mod profiles.** Named sets ("Vanilla", "Widescreen + HD textures", "Randomizer") per game, switched in one click, with **enable and
disable**, which Quiver lacks entirely. Mods are kept in a store Reclaw owns and placed into the game's mod folder (copies or reflinks;
symlinks need privileges on Windows). *Pros:* the feature that makes mods feel managed rather than dumped into a folder; backs up and
un-installs cleanly; Deck-friendly. *Cons:* the heaviest item here; every project lays out mods differently (`mods.path`, `layout` capture
only two shapes); archives that overwrite shared files need conflict handling. *Effort:* L. *Needs:* installer, provider clients.

**6. Conflict and dependency view.** Record which files each mod wrote; show "A overrides B on 3 files"; resolve Thunderstore dependencies.
*Pros:* the most common reason mods break. *Cons:* needs the file manifests from 5; GameBanana has no dependency metadata I know of.
*Effort:* M after 5.

**7. Collections.** A small shareable file listing a game version, mods and settings, rebuilt on any machine (Wabbajack's idea, scaled
down), so "my Deck setup for Ocarina of Time" is a link. *Pros:* reproducible, shareable, spreads the app by itself; links only, so no
redistribution of mod files. *Cons:* mods vanish or update and break a recipe; a shared recipe is an attack vector, so it needs a review
step and a clear "this will download N files from these sites"; moderation if a public index exists. *Effort:* L. *Needs:* 5.

**8. One-click install from the mod sites.** Both sites let a manager register a link handler so a button in the browser hands a mod to
the app. *Pros:* the least friction a player can have. *Cons:* the registration details for a third-party client are **unverified**; a
URL handler is an input from the open web and must be treated as hostile (confirm before downloading, allow-list the hosts).
*Effort:* S to M.

**9. Visual presets.** "Faithful", "Enhanced", "Max" for a game, a bundle of a texture pack, config values and a launch setting, tuned
to the hardware (Deck, integrated GPU, desktop). The launch-settings model already knows how a game takes resolution and window mode.
*Pros:* this is the project's promise (better graphics and performance) as a button. *Cons:* someone has to author and test presets per
game; hardware tuning is guesswork until reports exist (see 12). *Effort:* M per game, mostly curation.

## Discovery

**10. Recomp radar.** Find projects that Quiver's catalog has not listed: GitHub topics for static recompilation, new repositories that
look like recomps, a "suggest a project" button that drafts the catalog entry. *Pros:* serves your plan to add what Quiver lacks;
the catalog stays alive. *Cons:* GitHub search is rate-limited and noisy; some projects ship copyrighted assets and must not be listed, so
a human decides; legal exposure for a catalog that points at the wrong repository. *Effort:* M, mostly process.

**11. Real art for every game.** The catalog only gives each app an icon (116 from the SteamGridDB CDN, 103 from GitHub, the rest elsewhere).
A Steam-like shelf needs capsules, heroes and logos, which SteamGridDB serves through an API that needs a key (**unverified**: terms, limits
and attribution). *Pros:* the single biggest step toward looking like a store. *Cons:* a key to manage and rate limits to respect; community art
is uneven; fall back to our placeholders where missing. Our `reclaw.heroUrl`/`capsuleUrl` let us pin good picks. *Effort:* S to M.

**12. "Will it run on my machine?"** A ProtonDB-style rating per game and device class (Steam Deck, Bazzite desktop, Intel iGPU), starting
with the maintainers' own tested labels in the `reclaw` block and growing into opt-in reports. *Pros:* trust and the "Deck Verified" feeling;
tells players what to expect before a 2 GB download. *Cons:* a backend or a repository of reports to run; privacy design; a cold start
with no data. *Effort:* M, then ongoing.

## The first launch (the hardest step for a player)

**13. A game-file wizard.** Almost every recomp needs a game file the user already owns. Find it by scanning chosen folders and matching
hashes, say which version and region the project wants, convert the format if needed (byte order for N64 dumps), and remember it so every
recomp reuses it. This is the honest use for the install dialog's "choose your game file" step, which Quiver does not have. *Pros:* removes
the step where most people give up. *Cons:* needs each project's accepted hashes in the catalog; never downloads or links to game files,
which is a line to keep sharp; hash databases have their own terms (**unverified**). *Effort:* M. *Needs:* catalog fields.

## Feel (the gamer-themed, Deck-first part)

**14. A themed shell.** Per-system accent colours, optional CRT/scanline treatment, UI sounds and haptics, a short boot animation,
achievement-style toasts when an update lands, collection progress per system ("55 Nintendo 64 recomps"), playtime and a yearly recap.
*Pros:* delight and retention; the token pipeline already carries themes. *Cons:* the easiest place to burn weeks; motion and sound need off
switches; every themed surface is another thing to keep in step with both interfaces. *Effort:* S per piece; stop early.

**15. Steam integration.** Add each game to Steam as a non-Steam shortcut with art and launch options, and ship Steam Input layouts per system
(N64's C buttons on the right stick is a real annoyance). *Pros:* Deck game mode feels native; layouts are a gift to players. *Cons:*
Steam's shortcut file is a binary format that must be written while Steam is closed (Quiver has a writer to learn from); layouts are tuned
per game; Flatpak sandboxing; and a bad write corrupts a user's Steam library. *Effort:* M.

**16. A bug report that reaches the right developers.** One button gathers the log, versions and system summary into a prefilled issue
for the recomp project's own repository. *Pros:* maintainers get usable reports; players are not sent to guess where to complain.
*Cons:* privacy (opt-in, show what is sent); some projects do not use issues. *Effort:* S.

## Plumbing worth having

**17. Several catalogs, with provenance.** Quiver's, ours, a friend's, each shown with where an entry came from; ours signed so a tampered
copy is noticed. The format already supports many lists; the overlay in ADR 0009 lets us enrich theirs without copying it. *Cons:* a
signing key to guard. *Effort:* M.

**18. Library sync.** The library file and mod profiles (never game files) synced between a desktop and a Deck through a folder the user
already syncs, with no server of ours. *Pros:* no backend. *Cons:* conflict handling; paths differ per machine. *Effort:* M.

## What I would do first

In this order, because each makes the next cheaper and the first three fix real weaknesses rather than add features:

1. **Finish parity with the defects fixed on the way** (1 and the release-paging fix), because that is the foundation.
2. **11 and 17**: real art and the overlay catalog, so the app looks like a store with the 232 apps already in hand and we can start
   curating what Quiver lacks (including mod sources, which unlocks 5).
3. **13**, the game-file wizard: the highest drop-off in the whole journey for a new user, and it gives the existing install dialog a point.
4. **5**, mod profiles, as the headline feature, once there is an installer to build on.
5. Then pick from 3, 4, 12 and 15 by what players ask for.

I would leave 14 as seasoning to sprinkle between milestones rather than a milestone of its own.
