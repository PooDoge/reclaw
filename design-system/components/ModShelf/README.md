The Mods tab without a search. Rust: `reclaw_ui::desktop::pages::mods` (drawn), `reclaw_ui::mod_shelves` (pure: which mods, in which order, `Paging`).

* **Shelves, in order:** Installed (updates first), Most downloaded, Top rated (Thunderstore ratings, GameBanana likes), Recently updated, New releases. A shelf with nothing on it is left out. They follow the game chip and the site chip.
* **What is on a shelf is the site's own order.** The host asks each site for its mods in four orders (most downloaded 60, the others 20 each) and records each mod's place (`ModRanks`); a shelf shows the mods its order listed. Two sites on one shelf merge by the number the order is about (downloads, ratings, dates) where both give it, and by their places otherwise.
* **Header:** a 32px `bg-raised` tile with the shelf's icon in `accent` (check, download, star, refresh-cw, clock), the title in `heading`, the count in `mono` `ink-subtle`, what the order is in `meta`. **Show all N** (ghost, chevron-right) when there are more than shown.
* **Preview:** four mods in two columns on a wide window, three in one column on compact and phone.
* **Opened:** the whole shelf, 20 a page, **All shelves** (ghost, chevron-left) in the header's place of Show all, and a pager (Previous, "Page 1 of 3 · 1–20 of 57", Next) when it needs one. Search results page the same way.
* **Row:** title, then `meta`: author · site · version · downloads · ratings or likes · "updated 3 days ago", then the summary, and Install / Update / Remove.
