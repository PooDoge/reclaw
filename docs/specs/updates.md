# Updating from source

- last-verified: 2026-10-05
- owner-paths: scripts/update.sh, reclaw-app/build.rs, reclaw-app/src/host/update.rs, reclaw-app/tests/update_script.rs, reclaw-ui/src/about.rs

For a copy built from a git checkout (the development loop: code written elsewhere, run and tried on a Bazzite machine). ADR 0015.

## The script: `scripts/update.sh`

`scripts/update.sh [--no-build] [--check] [--profile debug|release]`. From the checkout it is in:

1. Refuses a detached head or a branch with no upstream, and says how to fix it (exit 2).
2. Refuses **local changes to tracked files** and lists them (exit 4). Untracked files are not in the way.
3. Fetches the branch's upstream (exit 5 if it cannot).
4. Compares. **Up to date**: says so. **Behind**: lists up to 30 new commits and fast-forwards (`--check` stops before this and
   says `available`). **Ahead**: nothing to pull, nothing changed. **Diverged**: exit 3, nothing changed.
5. Builds unless `--no-build`: `$RECLAW_BUILD_COMMAND`; else `scripts/bazzite-build.sh` when there is no `cargo`, it is not in a container
   and `distrobox` exists (with `--install` if `$RECLAW_RUNNING_EXE` is `~/.local/bin/reclaw`); else `cargo build --locked -p reclaw`
   (`--release` for the release profile; `$RECLAW_FEATURES` adds features). A build that exits non-zero or leaves no binary is exit 6, with
   the new source already in place.
6. Last line: `RECLAW_UPDATE: <result> <old> <new> <binary|none>`, `<result>` one of `updated`, `up-to-date`, `ahead`, `available`,
   `blocked-local-changes`, `cannot-fast-forward`, `fetch-failed`, `build-failed`.

It never resets, rebases, stashes, forces, or switches branch. The first time on a machine, check out the branch you want yourself.

## In the program

Settings, About: **Build** shows the commit (`a1b2c3d4e5 (release)`, `+local-changes` if the tree was dirty) and **Update from source**
runs the script in the background (`bash scripts/update.sh --profile <the running profile>`), sends every output line to the log, and
ends with a notice: updated ("Quit and start Reclaw again to use it", with the new commits and the binary), up to date, or one problem
notice per exit code with the script's own last lines. One update at a time. It does not restart the program.

`reclaw-app/build.rs` stamps the commit, the profile and the checkout's path into the binary. The row works only if that checkout still
exists and holds the script; otherwise it says the copy cannot update itself and how to get one that can.

## Verified, and not

Verified: every outcome above against real git repositories made for the test (`reclaw-app/tests/update_script.rs`), and the program's
handling of each exit code with a stand-in script. **Not verified**: the real build step (cargo is not run by the tests), the distrobox
path on Bazzite, replacing `~/.local/bin/reclaw` while it runs, and the row on a real desktop.
