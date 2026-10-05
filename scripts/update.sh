#!/usr/bin/env bash
# Bring this checkout up to date with its remote and rebuild, without ever discarding anything of yours.
#
#   scripts/update.sh                  # fetch, fast-forward, build (release)
#   scripts/update.sh --no-build       # only fetch and fast-forward
#   scripts/update.sh --profile debug  # build the debug binary (what `cargo run` runs)
#   scripts/update.sh --check          # only say whether there is anything new
#
# The same script runs when you choose Settings, About, "Update from source" in Reclaw, and prints one last line the program
# reads:  RECLAW_UPDATE: <result> <old commit> <new commit> <binary>
#
# What it will not do, by design: reset, rebase, stash, force anything, or change branch. It follows the branch this checkout is on
# (its upstream), and stops with an explanation if that cannot be done by a plain fast-forward (you have local changes to tracked files,
# you have commits the remote lacks, or the histories differ). Your own work stays exactly as it was.
#
# Building: $RECLAW_BUILD_COMMAND if set; else, on an immutable desktop (Bazzite) with no Rust of its own, scripts/bazzite-build.sh
# (inside a distrobox, installing over ~/.local/bin/reclaw when that is the copy you are running); else `cargo build --locked`.
#
# Exit codes: 0 done (or nothing to do), 2 cannot start (not a checkout, no upstream), 3 cannot fast-forward, 4 local changes,
# 5 fetch failed, 6 build failed.
set -u

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD=1 CHECK=0 PROFILE=release
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) BUILD=0 ;;
    --check) CHECK=1; BUILD=0 ;;
    --profile) shift; PROFILE="${1:-release}" ;;
    -h|--help) sed -n '2,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown option: $1 (see --help)" >&2; exit 2 ;;
  esac
  shift
done
case "$PROFILE" in debug|release) ;; *) echo "--profile must be debug or release, not '$PROFILE'" >&2; exit 2 ;; esac

say() { printf '==> %s\n' "$*"; }
finish() { printf 'RECLAW_UPDATE: %s %s %s %s\n' "$1" "${2:-none}" "${3:-none}" "${4:-none}"; }

cd "$ROOT" || { echo "cannot enter $ROOT" >&2; exit 2; }
git rev-parse --git-dir >/dev/null 2>&1 || { echo "$ROOT is not a git checkout, so there is nothing to update from." >&2; exit 2; }
command -v git >/dev/null || { echo "git is not installed." >&2; exit 2; }

branch="$(git symbolic-ref --quiet --short HEAD || true)"
[ -n "$branch" ] || { echo "HEAD is not on a branch (a detached checkout). Check out a branch first: git checkout <branch>" >&2; exit 2; }
upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{u}' 2>/dev/null || true)"
[ -n "$upstream" ] || { echo "The branch '$branch' does not follow a remote branch. Run: git branch --set-upstream-to=origin/$branch" >&2; exit 2; }
remote="${upstream%%/*}" remote_branch="${upstream#*/}"
old="$(git rev-parse --short=10 HEAD)"

say "this checkout: $ROOT"
say "branch $branch follows $upstream (now at $old)"

if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
  echo "You have local changes to tracked files, so nothing was changed:" >&2
  git status --short --untracked-files=no >&2
  echo "Commit or stash them (git stash), then run this again." >&2
  finish blocked-local-changes "$old"
  exit 4
fi

say "asking $remote for $remote_branch"
if ! git fetch --quiet "$remote" "$remote_branch"; then
  echo "Could not fetch from $remote. Check the network connection (and that the repository is reachable)." >&2
  finish fetch-failed "$old"
  exit 5
fi
target="$(git rev-parse FETCH_HEAD)"
new="$(git rev-parse --short=10 FETCH_HEAD)"
binary="${CARGO_TARGET_DIR:-$ROOT/target}/$PROFILE/reclaw"

if [ "$(git rev-parse HEAD)" = "$target" ]; then
  say "already up to date at $old"
  result=up-to-date
elif git merge-base --is-ancestor HEAD "$target"; then
  say "new commits:"
  git log --oneline --no-decorate -n 30 "HEAD..$target" | sed 's/^/    /'
  total="$(git rev-list --count "HEAD..$target")"
  [ "$total" -le 30 ] || say "    ... and $((total - 30)) more"
  if [ "$CHECK" = 1 ]; then
    say "an update is available ($total new commits); run without --check to apply it"
    finish available "$old" "$new"
    exit 0
  fi
  say "fast-forwarding"
  git merge --ff-only --quiet "$target" || { echo "The fast-forward failed; nothing was changed." >&2; finish cannot-fast-forward "$old" "$new"; exit 3; }
  result=updated
elif git merge-base --is-ancestor "$target" HEAD; then
  say "this checkout is ahead of $upstream (it has commits the remote does not); nothing to pull"
  result=ahead
else
  echo "Your checkout and $upstream have both moved on, so a plain fast-forward is not possible." >&2
  echo "Nothing was changed. Look at it with: git log --oneline --graph --all -n 20" >&2
  finish cannot-fast-forward "$old" "$new"
  exit 3
fi

if [ "$CHECK" = 1 ]; then
  finish "$result" "$old" "$new"
  exit 0
fi

if [ "$BUILD" = 0 ]; then
  finish "$result" "$old" "$(git rev-parse --short=10 HEAD)"
  exit 0
fi

# Always ask the build tool: when nothing changed it finishes at once, and when the source was updated by hand earlier it catches up.
in_container() { [ -f /run/.containerenv ] || [ -f /.dockerenv ]; }
status=0
if [ -n "${RECLAW_BUILD_COMMAND:-}" ]; then
  say "building: $RECLAW_BUILD_COMMAND"
  bash -c "$RECLAW_BUILD_COMMAND" || status=$?
elif ! command -v cargo >/dev/null && ! in_container && command -v distrobox >/dev/null && [ -x "$ROOT/scripts/bazzite-build.sh" ]; then
  extra=()
  # Replace the copy that is running when that is the installed one.
  [ "${RECLAW_RUNNING_EXE:-}" = "$HOME/.local/bin/reclaw" ] && extra+=(--install)
  say "building in the distrobox container (scripts/bazzite-build.sh ${extra[*]:-})"
  "$ROOT/scripts/bazzite-build.sh" ${extra[@]+"${extra[@]}"} || status=$?
  binary="${CARGO_TARGET_DIR:-$ROOT/target-bazzite}/release/reclaw"
elif command -v cargo >/dev/null; then
  flags=(--locked -p reclaw)
  [ "$PROFILE" = release ] && flags+=(--release)
  [ -n "${RECLAW_FEATURES:-}" ] && flags+=(--features "$RECLAW_FEATURES")
  say "building: cargo build ${flags[*]}"
  cargo build "${flags[@]}" || status=$?
else
  echo "There is no way to build here: cargo is not installed. On Bazzite install distrobox's build with scripts/bazzite-build.sh." >&2
  finish build-failed "$old" "$(git rev-parse --short=10 HEAD)"
  exit 6
fi
if [ "$status" -ne 0 ] || [ ! -x "$binary" ]; then
  echo "The build failed (exit $status). The new source is in place; fix the error above and run this again." >&2
  finish build-failed "$old" "$(git rev-parse --short=10 HEAD)"
  exit 6
fi
say "built $binary"
finish "$result" "$old" "$(git rev-parse --short=10 HEAD)" "$binary"
