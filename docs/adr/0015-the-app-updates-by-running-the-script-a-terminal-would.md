# 0015 The app updates by running the script a terminal would

- status: accepted
- date: 2026-10-05
- spec: ../specs/updates.md

## Context

The code is written and tested in one place (a cloud sandbox with no GPU, no gamepad, no GNOME) and run in another (a Bazzite
desktop). The loop is: pull, rebuild in a distrobox, restart. Each step by hand is friction, and the loop is how every hardware-only
problem gets found. Settings could offer the loop as a button.

## Decision

* One script, `scripts/update.sh`, does it from a terminal and from Settings, About, "Update from source". The program runs the script and
  turns its output (to the log line by line) and its final `RECLAW_UPDATE:` line into a notice.
* The script **only fast-forwards the branch the checkout is on**. It refuses local changes to tracked files, a diverged history and a
  detached head, says why, and changes nothing. It never resets, rebases, stashes, force-anything or switches branch.
* It builds with `$RECLAW_BUILD_COMMAND`, else `scripts/bazzite-build.sh` on an immutable desktop with no Rust (replacing the installed copy
  when that is the one running), else `cargo build --locked` in the profile that is running.
* The program does not restart itself. It says to quit and start again; the About page shows the commit that is running.
* A copy not built from a checkout (the build stamps the checkout's path; it must still exist and hold the script) says so.
* Development stays where it is. The sandbox runs the headless UI tests, a virtual display and the live services; the Bazzite machine
  covers what only hardware can.

## Rejected

* **Doing git and cargo in Rust (libgit2, `cargo` as a library).** Duplicates a script that is needed in a terminal anyway, and adds a
  large dependency to do less safely.
* **Updating by downloading a built binary.** There are no releases yet.
* **Restarting automatically.** A window that vanishes and returns while someone is looking at it is worse than a message, and the old
  binary keeps working until they choose.
* **`git pull` (merge) or `git reset --hard` to "just make it match".** The second destroys local work; the first creates merges in a
  checkout nobody meant to commit in. Fast-forward or stop.
* **Checking for updates in the background.** A launcher that contacts a repository on its own is something to add on purpose, with a
  setting; the button is explicit.
* **Moving the editing to the local machine.** Considered. It would put the build next to the hardware but give up the cloud side's
  parallel headless tests, the virtual display and the clean network; and a one-command update removes most of the friction that
  motivated it. Revisit if the Bazzite machine becomes the place where most failures are found.
