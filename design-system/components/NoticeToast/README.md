The newest notification as a card at the bottom right of Deck mode, with the two holds that act on it. Rust: `reclaw_ui::deck::NoticeToast` (`deck/widgets/notice.rs`); the queue and its rules are `reclaw_ui::notices` (a repeat replaces, the newest is shown, dismiss one or all).

* **Hold X** (900 ms) opens the notice's details; **hold Y** (1200 ms) dismisses every notice. The destructive one is longer. Durations are `DETAILS_AFTER` and `DISMISS_ALL_AFTER` in `notices/hold.rs`.
* The button's glyph sits inside a ring (`HoldRing`: 44px, a 4px `line-strong` track and an `accent` arc from the top, clockwise). The ring runs on its own animation clock for the hold's length; the pad reader's clock decides when the hold completes, and the two agree to within a frame.
* A tap still does what the button always does (X is Options, Y is Search); while a toast is up a tap is decided when the button comes up, so the hold can be told apart; a hold completes at its time, while the button is still down. The toast is **not focusable**: it never takes the pad from the page.
* Tone: update available `warn` + download; finished `ok` + check; failed `danger` + triangle-alert. The same icon and color table as the card indicator.
* Slides in from the right (280ms, expo-out) when a new notice arrives. The toast is 460 x 132 (Rust constants `TOAST_W`, `TOAST_H`; not yet tokens).

Unverified: the hold times on a real controller. They are untested outside the keyboard stand-in and the pad reader's tests.
