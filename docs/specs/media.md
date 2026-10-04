# Artwork, screenshots and READMEs

- last-verified: 2026-10-04
- owner-paths: reclaw-media/src/**, reclaw-media/tests/**, reclaw-ui/src/media/**, reclaw-ui/src/readme/**, reclaw-ui/src/components/remote_art.rs, reclaw-ui/tests/ui/media.rs

What Reclaw fetches from the internet to show, and the rules it fetches by. Everything fetched is written by strangers.

## The rule

**Nothing the user sees is fetched by the UI toolkit.** Every picture and document goes through `reclaw-media`, which applies one set
of limits; the toolkit's `remote-asset` feature is switched off, so even the stock markdown viewer cannot fetch on its own.

## What is allowed (`source`)

https only, the standard port, no user name or password, no address on the local network (by name: `localhost`, names with no dot,
`.local` `.lan` `.internal` ...; by number: loopback, private, link-local, shared, documentation, multicast ranges, in IPv4 and
IPv6, including IPv4 mapped into IPv6). The same check runs on **every redirect**, so a public address cannot send the request
somewhere private. DNS rebinding is not defended against.

## What happens to a download (`cache`, `store`)

* Size capped per kind (8 MiB for pictures, 1 MiB for text), read with a hard stop, 20 s overall and 8 s to connect.
* **What a file is comes from its bytes** (`sniff`), never from the server: a picture must be PNG, JPEG, GIF, WebP or SVG; text must be
  UTF-8 with no NUL bytes. Anything else is refused and not stored. A picture's size is read from its header (`dimensions`) so a page
  can reserve room.
* One file per address under `<cache>/media/`, written to a temporary name and renamed. Fresh for a day (no request at all), then
  fetched again; when that fails the old file is served for up to 30 days. Trimmed to 256 MiB least recently used first, now and
  then. A failure is remembered for ten minutes so a dead link is not retried on every redraw.
* `MediaHub`: four worker threads and a queue; asking again for an address in flight joins the first request.
* `SSL_CERT_FILE` adds certificate authorities for networks that re-sign HTTPS.
* The user can switch it all off: Settings > Library > *Download artwork and READMEs*. Pictures then stay placeholders.

## In the UI

`use_remote_file(url, want)` turns an address into *off / loading / ready / failed*; `RemoteArt` shows a placeholder until the file
is on disk and keeps it if the file never arrives. A component using the hook is keyed by the address. Used for banners, capsules,
Deck tiles and the screenshot strip. With no `Shell` around it (a component preview) it fetches nothing.

## READMEs (`reclaw-media::readme`, `reclaw-ui::readme`)

The README of the project's repository (`raw.githubusercontent.com` or GitLab's raw address, on the default branch) is cut into
blocks. The stock markdown viewer draws the prose, restyled with Reclaw's colors; nothing of the toolkit is forked.

* **Prose** stays markdown. Relative links become absolute links to the file's web page; `javascript:`, `mailto:` and `#anchors` become
  plain text. **An image inside prose becomes a link to the image**, because the stock viewer would fetch it itself.
* **Pictures** that sit on a line of their own (a logo, a screenshot, a row of badges), written as markdown or as HTML, become
  `Pictures` blocks: fetched by us, laid out with their true proportions (the README's own width wins), pressable when the README
  wrapped them in a link. A relative path cannot leave the repository.
* **Badges** (shields.io and a dozen other sites, or any `badge.svg`) are read from their address and drawn as a native two-part
  chip, because the toolkit's SVG drawing has no fonts and the text would be missing. A badge whose words are not in its address
  shows its alt text.
* **HTML**: the subset READMEs use (centered logos and headings, `<details>` shown open, lists, `<pre>`, links, line breaks) is
  lowered to blocks. `script`, `style`, `iframe`, `object`, `form`, inline `svg`, `video` and the rest are dropped **with their contents**;
  event handlers and unknown attributes are never read.
* **Mermaid** fences become a `Diagram` block, shown as source for now.
* The page shows the start (about 1800 characters of reading) and a *Show the whole README* button.

## Not built / not verified

Drawing diagrams; animated GIFs; SVG pictures that contain text; READMEs in other formats (`.rst`, `.adoc`) and docs beyond the
README; a settings row to clear the cache or show its size; Deck mode has no README view. Verified against the real internet once
(`cargo test -p reclaw-media --test real_network -- --ignored`: a real README and image over TLS); not verified at scale or on a
slow or captive network. Prose uses the stock viewer's fonts, not Reclaw's.

## Tests

`reclaw-media` unit tests for each module (126), a scrambled-markup test that nothing panics, the ignored real-network test, and
`reclaw-ui/tests/ui/media.rs` against a fake web: placeholder and picture, one request however many redraws, switched off, a 404, the
README with native badges and pictures, collapse and expand, a hostile README.
