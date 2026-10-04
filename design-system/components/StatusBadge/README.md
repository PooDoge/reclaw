One badge per game state. Every badge carries a word and, except Not installed, an icon, so state never depends on hue: the green and red here differ in lightness and by label.

States: `installed` (ok), `update` (warn), `installing` (info), `failed` (danger), `available` (neutral), `needsfile` (warn). **Needs your game file** is specific to recompilation: the project ships no copyrighted assets, so the user must supply their own ROM or disc image before Install can finish. Never offer to download it.

Freya: a custom `rect()` with a Lucide icon and a `label()`; fill and text come from the `*-bg` and matching status token.
