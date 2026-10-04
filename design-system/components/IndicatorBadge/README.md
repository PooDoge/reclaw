The icon and short text of a game's background work: "34%", "Building", "Updated", "Update ready". Rust: `reclaw_ui::components::IndicatorBadge`; icon and color per kind in `indicator_look` (below); what a game's indicator is comes from `reclaw_ui::activity::indicator_for`, which is pure and tested.

| Kind | Icon (Lucide) | Color | Meaning |
|---|---|---|---|
| UpdateAvailable | download | `warn` | A newer version exists; nothing has started |
| Queued | clock | `ink-muted` | Waiting for its turn |
| Downloading | arrow-down-to-line | `accent` | Bytes are arriving |
| Installing | package | `info` | Verifying, building or extracting |
| Done | check | `ok` | Finished this run; stays until the app restarts, with what changed |
| Failed | triangle-alert | `danger` | Stopped; the notice says why |
| Mods | puzzle | `accent` | Only mods are downloading for this game |

Never by color alone: every kind has its own icon and a word or a number. In the desktop sidebar the Updates section lists the games that have one of these, with the same icons, and finished updates keep their row, with the changelog, until Reclaw restarts.
