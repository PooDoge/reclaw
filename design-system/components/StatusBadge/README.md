One badge per game state. Every badge carries a word and, except Not installed, an icon, so state never depends on hue: the green and red here differ in lightness and by label.

States: `installed` (ok), `update` (warn), `installing` (info), `failed` (danger), `available` (neutral). **Not installed** is the neutral one: it has no icon, so state never depends on hue alone. A badge is always a word.

Freya: a custom `rect()` with a Lucide icon and a `label()`; fill and text come from the `*-bg` and matching status token.
