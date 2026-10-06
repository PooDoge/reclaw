What every desktop "Failed" indicator does. Rust: `reclaw_ui::components::FailedHint`, used by `DownloadItem` (the stage label), `StatusBadge` (`.failure(text)` and `.on_failure(handler)`), `HeroHeader` (`.failure(text, handler)`) and `GameCapsule` (`.failure(text)`, tooltip only). The text and the target come from `reclaw_ui::activity::{hint_for_game, hint_for_activity}` (pure, tested).

* **Hover:** Freya's `TooltipContainer` after its 500 ms delay, one line: the reason cut to 80 characters at a word, with "..." (`short_reason`). Below the badge; to the left of a Downloads label, which sits at the right edge of its row.
* **Press:** opens `FailureLog` for the job. A capsule's badge has the tooltip only: pressing a capsule opens the game, and its page's badge opens the log.
* **Earlier run:** a game that says Failed after a restart has no job left on the board. The tooltip says the last install did not finish and its log is in the log folder; a press opens the log folder (`Effect::OpenLogFolder`).
* The label gains the triangle-alert icon so it reads as something to act on, not only a word. Pointer cursor when pressable; an accessible name "Failed: <reason>. Show the log".

Deck mode has no hover. Its failure notice carries the same details; the log view is not on Deck yet.
