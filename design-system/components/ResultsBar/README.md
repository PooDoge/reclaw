The line above a search's results. Rust: `results_bar`, `no_results` and `highlighted` in `reclaw_ui::desktop::pages::common`.

* `bg-panel`, a 3px `accent` bar on the left, the magnifier in `accent`; "3 results for “camera”" in `label` `ink`, where it looked ("in mods") in `meta` `ink-subtle`, and **Clear search** (ghost, x). In the Library's sidebar (`narrow`) the place goes and Clear is its icon.
* **Nothing found** replaces the list: the magnifier, "Nothing in the catalog matches “zzzz”", a hint naming what else narrows the list (platform, site, filter), and Clear search (secondary). An empty list is explained, never blank.
* **Highlight:** in mod results the words that matched are `accent` and bold in the title (`search::highlight`, case-insensitive, in the title's own letters).
* **Settings** results are the real rows, working, under their section's title in an `accent` eyebrow: change a setting without going to its section (`search::filter_schema`; a row is found by its label, description or choices, or by its section's or group's title).
