#!/usr/bin/env bash
# Runs the real launcher under a virtual X server with a window manager, and checks that the
# window behaves: custom title bar buttons, dragging, resize bands, Deck mode fullscreen, and that the
# size and place survive a restart. This is the only check that exercises the real winit window; the
# headless tests never open one.
#
#   cargo build -p reclaw && scripts/x11-smoke.sh
#
# Needs: Xvfb, openbox, xdotool, x11-utils (xwininfo, xprop), libxkbcommon-x11.
#   Debian/Ubuntu: apt-get install xvfb openbox xdotool x11-utils libxkbcommon-x11-0
#
# What it cannot tell you: anything about Wayland, a compositor (transparency, rounded corners), more
# than one monitor (a virtual X server has one), or a high-DPI screen.
set -u
BIN="${RECLAW_BIN:-$(dirname "$0")/../target/debug/reclaw}"
DISPLAY_NUM="${RECLAW_X11_DISPLAY:-79}"
SHOTS="${RECLAW_SHOTS:-/tmp/reclaw-x11-shots}"
export DISPLAY=":$DISPLAY_NUM"
export RECLAW_HOME="$(mktemp -d)"
# The window is what is under test, not the network: an index address the network layer refuses outright keeps the run hermetic.
export RECLAW_CATALOG_INDEX="https://127.0.0.1/index.json"
mkdir -p "$SHOTS"

failures=0
check() { # check <description> <expected> <actual>
  if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: expected '$2', got '$3'"; failures=$((failures + 1)); fi
}
cleanup() { kill "${APP:-}" "${OB:-}" "${XVFB:-}" 2>/dev/null; rm -rf "$RECLAW_HOME"; }
trap cleanup EXIT

[ -x "$BIN" ] || { echo "no binary at $BIN; build it first"; exit 2; }
# A server that was killed rather than stopped leaves its lock behind, and the next one refuses to start.
rm -f "/tmp/.X${DISPLAY_NUM}-lock" "/tmp/.X11-unix/X${DISPLAY_NUM}"
Xvfb ":$DISPLAY_NUM" -screen 0 2560x1080x24 >"$SHOTS/xvfb.log" 2>&1 & XVFB=$!
for _ in $(seq 1 20); do xwininfo -root >/dev/null 2>&1 && break; sleep 0.5; done
xwininfo -root >/dev/null 2>&1 || { echo "the X server did not start; see $SHOTS/xvfb.log"; exit 2; }
openbox >/dev/null 2>&1 & OB=$!
sleep 1

launch() { "$BIN" >"$SHOTS/app.log" 2>&1 & APP=$!; sleep 10; WID=$(xdotool search --class "dev.reclaw.Reclaw" | head -1); }
field() { xwininfo -id "$WID" | awk -v k="$1" '$0 ~ k {print $NF}'; }
size() { echo "$(field Width)x$(field Height)"; }
place() { echo "$(field 'Absolute upper-left X'),$(field 'Absolute upper-left Y')"; }
wm_state() { xprop -id "$WID" _NET_WM_STATE | sed 's/.*= *//'; }
# Freya decides what is under the pointer from motion events, so a click needs a move before it.
click_at() { xdotool mousemove "$1" "$2"; sleep 0.4; xdotool mousemove_relative 1 0; sleep 0.3; xdotool mousedown 1; sleep 0.15; xdotool mouseup 1; sleep 1.5; }
drag_from() { # drag_from <x> <y> <dx> <dy>
  xdotool mousemove "$1" "$2"; sleep 0.5; xdotool mousemove_relative 1 0; sleep 0.3
  xdotool mousedown 1; sleep 0.3
  xdotool mousemove_relative -- $(($3 / 2)) $(($4 / 2)); sleep 0.3
  xdotool mousemove_relative -- $(($3 / 2)) $(($4 / 2)); sleep 0.3
  xdotool mouseup 1; sleep 1.5
}

launch
check "the window opens at the default size" "1280x800" "$(size)"
check "it has the app id desktop environments group by" "dev.reclaw.Reclaw" "$(xprop -id "$WID" WM_CLASS | sed 's/.*"\(dev.reclaw.Reclaw\)".*/\1/')"
import -window root "$SHOTS/1-start.png"

X0=$(field 'Absolute upper-left X'); Y0=$(field 'Absolute upper-left Y'); W=$(field Width); H=$(field Height)
click_at $((X0 + W - 69)) $((Y0 + 18))
check "the maximize button maximizes" "_NET_WM_STATE_MAXIMIZED_VERT, _NET_WM_STATE_MAXIMIZED_HORZ" "$(wm_state)"
# Maximized, the window starts at the screen's corner, so aim from where it is now.
MX=$(field 'Absolute upper-left X'); MY=$(field 'Absolute upper-left Y'); MW=$(field Width)
click_at $((MX + MW - 69)) $((MY + 18))
check "pressing it again restores" "1280x800" "$(size)"
X0=$(field 'Absolute upper-left X'); Y0=$(field 'Absolute upper-left Y')

click_at $((X0 + W - 115)) $((Y0 + 18))
check "the minimize button hides the window" "_NET_WM_STATE_HIDDEN" "$(wm_state)"
xdotool windowmap "$WID"; xdotool windowactivate "$WID" 2>/dev/null; sleep 2

BEFORE_X=$X0; BEFORE_Y=$Y0
drag_from $((X0 + 300)) $((Y0 + 18)) 300 150
# The window manager takes over a drag after its first motion, so the move is a little short of the
# pointer's: the check is that it went the way it was dragged.
X0=$(field 'Absolute upper-left X'); Y0=$(field 'Absolute upper-left Y')
[ "$X0" -gt "$BEFORE_X" ] && [ "$Y0" -gt "$BEFORE_Y" ] && moved=right-and-down || moved="$(place)"
check "dragging the title bar moves the window" "right-and-down" "$moved"

drag_from $((X0 + W - 3)) $((Y0 + H / 2)) -120 0
check "dragging the right edge resizes it" "1160x800" "$(size)"
W=$(field Width)
drag_from $((X0 + W - 3)) $((Y0 + H - 3)) 100 50
check "dragging the corner resizes both ways" "1260x850" "$(size)"
W=$(field Width); H=$(field Height)
import -window root "$SHOTS/2-moved-and-resized.png"

SAVED_PLACE=$(place)
xdotool key F10; sleep 3
check "Deck mode fills the screen" "2560x1080" "$(size)"
check "and asks the window manager for fullscreen" "_NET_WM_STATE_FULLSCREEN" "$(wm_state)"
import -window root "$SHOTS/3-deck.png"
xdotool key F10; sleep 3
check "leaving Deck mode gives the window back" "1260x850" "$(size)"
check "in the same place" "$SAVED_PLACE" "$(place)"

click_at $((X0 + W - 23)) $((Y0 + 18))
sleep 2
if kill -0 "$APP" 2>/dev/null; then check "the close button ends the program" "exited" "still running"; else check "the close button ends the program" "exited" "exited"; fi
grep -q 'size = \[' "$RECLAW_HOME/config/settings.toml" 2>/dev/null && saved=yes || saved=no
check "and the window was saved on the way out" "yes" "$saved"

launch
check "the next start reopens at the saved size" "1260x850" "$(size)"
check "and the saved place" "$SAVED_PLACE" "$(place)"

echo; [ "$failures" -eq 0 ] && echo "all checks passed (screenshots in $SHOTS)" || echo "$failures check(s) failed (screenshots in $SHOTS)"
exit "$failures"
