The picture at the top of a game's page. Rust: `reclaw_ui::components::BannerArt`; the colours come from `reclaw_ui::banner`. The catalog mostly carries icons, so a page can rarely count on a wide picture. It uses the best one there is, in this order:

1. **The catalog's banner** (`reclaw.heroUrl`), trusted as it is.
2. **A picture from the project's README**: up to three, best first, ranked from the text alone (words like banner, header, screenshot; badges, buttons and sponsor images are never candidates). One is used only when its real size is at least 480 px wide and 1.6:1 or wider, so an icon is not blown up and a square is not cropped to a stripe.
3. **A generated banner**: the game's own colour (a hue taken from its title with FNV-1a, so it is the same on every run and every machine) mixed into the theme's background, with the game's icon blurred behind the icon itself, kept clear of the title strip. It needs no network, so the box is never empty and never a label.

Whatever fails to arrive (offline, a dead link, something that is not an image, "Download artwork and READMEs" switched off) falls through to the next step. Pictures are cropped to the box like cover art. In this mock the icon is the title's first letter, standing in for the catalog icon.

Don't: show a placeholder label such as HERO on a real page; stretch a small or square picture to fill the banner; put the title on the art without the solid strip (`HeroHeader` does this).
