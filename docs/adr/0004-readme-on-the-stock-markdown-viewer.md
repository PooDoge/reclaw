# 0004 READMEs are drawn on the stock markdown viewer, not a fork and not an HTML engine

- status: accepted
- date: 2026-10-04
- spec: ../specs/media.md

## Context

A project's README is the best description of it. The toolkit's markdown viewer draws prose, lists, tables and code, and drops what
GitHub READMEs rely on: raw HTML (centered logos, `<details>`), relative image paths, badges, task lists. A fork of the viewer exists
in another of the owner's projects (oxide-code) with more features; the owner asked to bring the learnings here without forking,
so that moving to a newer Freya stays easy. The toolkit also has an HTML renderer (Blitz).

## Decision

Cut the README into blocks **before** the viewer sees it (`reclaw-media::readme`, pure and tested): prose stays markdown with links
made absolute and images turned into links; pictures, badge rows and diagrams become blocks the UI draws with its own components;
the HTML READMEs use is lowered to those blocks. The stock `MarkdownViewer` draws the prose, restyled through its public theme.

## Rejected

* **Forking `freya-markdown`** as oxide-code did. Its extra features (selectable prose that copies as source, inline chart blocks,
  bare-path chips) need the viewer's internals, and a fork must be rebased on every Freya release. What READMEs need is outside
  the viewer, and the pre-split reaches it with public APIs only.
* **The toolkit's HTML renderer** (`freya-html`, Blitz). It would render raw HTML and remote SVG, but it fetches every subresource
  itself with no rules (ADR 0003), a clicked link navigates *inside* the viewer so a README link could load an arbitrary page in the
  app, it reports no height, and it pulls in the Blitz and Stylo stacks. Evaluated by reading its source; not adopted.
* **Copying oxide-code's line-based fence splitter.** It cuts fences nested in lists, which READMEs have; the offset iterator of the
  markdown parser cuts only top-level blocks.
* **Rendering badges from their SVG.** The toolkit's SVG drawing has no fonts, so the label text is missing. The label, message and
  colour are in the common badges' addresses, so they are drawn as native chips.

## Consequences

Prose does not support strikethrough, task-list boxes or footnotes (shown as typed). Selecting and copying README text is not
supported. A README with unusual HTML loses what is not in the whitelist, and loses it silently. Mermaid is shown as source until
diagrams are drawn; the parsers in oxide-code are Freya-free and could be ported.
