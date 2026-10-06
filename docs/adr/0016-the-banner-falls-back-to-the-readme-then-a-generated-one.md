# 0016 The banner falls back to the README, then to a generated one

- status: accepted
- date: 2026-10-05
- spec: ../specs/media.md

## Context

The game page's wide banner could only come from the optional `reclaw.heroUrl` block of a catalog entry. The real community catalog
has none: all 232 entries carry an icon (`appIconUrl`) and nothing else, so the banner was an empty box on every page. While looking
at that, a second fault showed: the title strip over the banner is drawn at a shallower depth of the element tree than the picture
below it, and this toolkit paints deeper elements over shallower ones whatever the order of siblings, so a real picture would have
covered the title.

## Decision

The banner is the first of these that works:

1. the catalog's own `heroUrl`, trusted as it is;
2. up to three pictures from the project's README, ranked from the README's text alone (`reclaw-media::readme::banner`: banner,
   header, hero, cover, screenshot... words win; badges, buttons, sponsor links and avatars never qualify; a size the README states
   rules a picture out without fetching it), then accepted only when the fetched file's real size is at least 480 pixels wide and 1.6
   times wider than tall (`fits_banner`), because a 3:1 crop of a square logo shows its middle strip;
3. a generated banner: a gradient from a hue taken from the game's name (FNV-1a, so it never changes between runs or Rust releases)
   mixed into the theme's own background, with the icon blurred behind a crisp copy of itself. It needs no network at all, so with
   downloads switched off a game still has a banner.

Each step is a component that calls its hook once and hands over to the next on failure, so hooks stay unconditional. The README is
fetched through the same `use_readme` hook the README section uses; the hub fetches an address once, so the page does not ask twice.
The title strip is lifted above the art with an explicit layer.

## Rejected

* **GitHub's social preview card (`opengraph.githubassets.com`).** It cannot be told apart from the automatic card, which repeats the
  repository name the title strip already shows and carries counts that go stale; it also adds a host to the list of those Reclaw needs.
* **Cropping the icon to fill the banner.** A 256 pixel icon blown up to 1100 is a blur with a visible grid.
* **Accepting any README picture.** Logos and badges are the first pictures in most READMEs; they crop to a stripe.
* **Reading the dominant colour of the icon.** It needs decoding the picture in the UI thread; a hue from the name is stable, instant
  and works when the icon cannot be fetched.
* **Asking for a `heroUrl` in the catalog.** The catalog belongs to its maintainers; Reclaw reads it as it is (ADR 0009).

## Consequences

Opening a game page fetches its README even when the user never scrolls to it. It is capped at 1 MiB, cached for a day and shared
with the README section. A README whose first wide picture is a screenshot of a menu will show that menu as the banner; that is
usually fine, and the catalog's `heroUrl` is the way to choose.
