# 0006 Press-and-hold is decided in the pad reader, and a tap keeps its meaning

- status: accepted
- date: 2026-10-04
- spec: ../specs/notices-and-holds.md

## Context

A notification toast offers two actions on buttons that already do something else (X opens Options, Y searches): hold X for
details, hold Y to dismiss everything, each with a ring that counts down in the button's glyph.

## Decision

`HoldTracker` is a pure state machine with the clock passed in. The pad reader runs it while the UI has said a toast is up
(`Effect::NoticeHolds`): a held button acts on *release*, as a tap (its ordinary action, after clearing the ring) if released early,
and as the hold if the time passed, once. The keyboard uses the same tracker so Deck mode is testable without a pad. The destructive
hold is the longer one.

## Rejected

* **Timers in the UI.** The UI would need press and release events from the pad, which the reader turns into actions; and timing
  there is subject to frame hitches.
* **Dedicated buttons for details and dismiss.** There are none to spare on a pad, and the toast must not take focus from the page.
* **Acting on press and cancelling on a long hold.** A tap would fire its ordinary action immediately and the hold could not undo it.
* **Holds always on.** X and Y would act on release everywhere, adding a delay to every Options press when nothing is held.

## Consequences

While a toast is up, X and Y act on release rather than press. The time values (0.9 s, 1.2 s) are untested on a real controller.
