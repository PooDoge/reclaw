The dialog a press on "Failed" opens. Rust: `FailureLogView` in `reclaw_ui::desktop::dialogs::failure_log`, a `Dialog` of kind Form with `.wide(true)`: a 760px popup on desktop, a full-screen page with Back on touch. Data: `reclaw_ui::activity::report(board, id)`.

* Title "<job> failed"; the whole reason in `body` `ink`; each detail (the cause, what to do, where the log file is) in `meta` `ink-muted`.
* **Log:** the lines the job wrote while it ran (`Activity::log`, recorded by `reclaw_log::record` around the install, ADR 0019), oldest first, `mono` on `bg-deep` in a `line` box. Warnings in `warn`, errors in `danger` (`line_tone` reads the level column, not the words). The box grows with the lines up to 320px, then scrolls. At most the last 400 lines; when more were written the first line says how many were dropped.
* Same level and redaction as the log file: Settings, Diagnostics, Log detail "Detailed" records more. No token is ever in a line.
* Actions: "Open log folder" (ghost, folder-open) for the whole file, "Close" (secondary). Back and Escape close it before the page.

Don't: show it for a running or finished job (there is no report); put the log anywhere a stranger can read it without the person choosing to share it.
