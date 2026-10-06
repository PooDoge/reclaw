# Background work, notifications and press-and-hold

- last-verified: 2026-10-06
- owner-paths: reclaw-ui/src/activity/**, reclaw-ui/src/notices/**, reclaw-input/src/hold.rs, reclaw-input/src/mapper.rs, reclaw-ui/src/deck/app/key_holds.rs, reclaw-ui/src/deck/widgets/notice.rs, reclaw-ui/src/deck/widgets/card_progress.rs, reclaw-ui/tests/ui/deck_notices.rs, reclaw-ui/src/components/failed_hint.rs, reclaw-ui/src/desktop/dialogs/failure_log.rs, reclaw-ui/tests/ui/desktop_failures.rs

How downloads, installs, updates and mod downloads are tracked and shown, and how Deck mode tells the user and lets them act with a
button held down.

## Activity

The host reports `ActivityEvent`s (started, progress, finished, failed, log, cancelled, dismissed); `ActivityBoard::apply` is the only
way the list changes. A finished or failed job **stays on the board until the app restarts**, so an automatic update leaves its
changelog to read. `indicator_for(board, game)` is what a card or row shows for one game: update available, queued, downloading,
installing, done, failed, or mods downloading, with a label ("34%", "Building") and a progress fraction when the size is known.
`sidebar_entries` orders the desktop's Updates section: active, then failed, then waiting, then done.

## Where it shows

* **Desktop:** the Updates section of the Library sidebar, the Downloads page and its tab badge, the status bar.
* **Deck:** a chip at the top left of a game's card (icon and "34%" / "Building" / "Update ready") and a bar along the bottom of its
  art: filled when the size is known, a sliding segment when it is not, full and green when finished, absent when only an update
  waits. Both sit inside the art, so they scale with the focused card.
* **Meaning is never colour alone:** every state has its own icon and word.

## Why a job failed (desktop)

A failed job carries a one-line reason, details (the cause, what to do, where the log is) and **the lines it wrote to the log**
(`Activity::log`, sent by the host as `ActivityEvent::Log` just before `Failed`; ADR 0019). Every "Failed" indicator explains itself
(`activity::failure`, `components::FailedHint`):

* **Hover:** a tooltip with the reason, cut to 80 characters at a word (`short_reason`). On the Downloads rows (both the Downloads page and
  the Library's list), the Failed badge of the Library hero and the game page, and the badge on a Library capsule.
* **Press** (not on a capsule, whose press opens the game): a dialog, "<title> failed", with the whole reason, the details, and the
  job's log in a scrolling box, warnings and errors in colour (`line_tone`), and "Open log folder". A popup on desktop, a full-screen
  page on touch; Back and Escape close it first.
* **A failure from an earlier run** (the game says Failed, the board has no job for it after a restart): the tooltip says the last
  install did not finish and its log is in the log folder; pressing opens the log folder.
* Deck mode has no hover; its failure notice already carries the details (Notifications below). The log view is not on Deck yet.

## Notifications

Finishing or failing a job, a game newly needing an update, or a mod installing pushes a `Notice` (`store::reduce` decides when).
`Notices` keeps them: a repeat replaces, the newest is the one shown, failures wait for the user. In Deck mode the newest is a
**toast** at the bottom right (`NoticeToast`) while nothing else is on screen (`DeckState::toast_visible`).

## Press and hold

The toast offers two holds: **hold X for details** (0.9 s) opens the notice's full text over the page; **hold Y to dismiss all**
(1.2 s). The button's glyph sits in a ring that fills while it is held. The destructive one needs the longer hold.

* The gesture is `reclaw_input::HoldTracker` (pure; time is passed in). The pad reader runs it only while the UI has asked for it
  (`Effect::NoticeHolds(true)`, sent exactly while a toast is up), so X and Y act on press, as always, the rest of the time.
* **A tap still does the button's ordinary job, on release.** While holds are on, a press cannot yet be told from the start of a
  hold, so the tap is decided when the button comes up: released early, the ring clears and the ordinary action runs (X opens
  Options, Y searches). A hold that reaches its time **completes at that moment, while the button is still down**, once, and the
  release after it does nothing.
* **A key held on a keyboard repeats** as more "key down" events and Freya does not say which are repeats, so `key_holds.rs`
  remembers which keys are down and ignores a second down with no up between. (Before this, holding Y past its time dismissed the
  toast and the repeats that followed each opened Search; found by running the real window with `xdotool`.) A pad has no repeat.
  If a key-up is ever lost (focus taken mid-hold) the next press of that key is ignored once.
* **The keyboard does the same** (`key_holds.rs`, keys `x` and `y`, over the same tracker) so Deck mode can be driven and tested
  without a pad.
* The ring runs on its own clock for the length of the hold; the reader's clock decides when it completes. They agree to within a frame.
* `DeckState` re-evaluates whether holds are wanted whenever the notice list changes, so dismissing the last notice turns them off.

## Not verified

The hold timing with a physical controller; whether 0.9 s and 1.2 s feel right in the hand.

## Tests

`activity/tests/**`, `notices/tests/**`, `reclaw-input` (hold tracker, mapper), `deck/state/tests/notices.rs`, `deck/app/key_holds.rs`,
and `tests/ui/deck_notices.rs` (the toast, the holds from the pad and the keyboard, taps, cards, snapshots).
