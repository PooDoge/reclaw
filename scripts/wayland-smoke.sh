#!/usr/bin/env bash
# Runs the real gallery on native Wayland, on a headless sway (a real compositor), and checks that the window can be
# resized: the resize cursor the app asks for at each edge and corner, and a real drag, at several display scales.
# This is the check that found the resize bands missing the right and bottom edges on a scaled display (ADR 0008);
# the X11 rig (x11-smoke.sh) runs at scale 1 and could not.
#
#   cargo build -p reclaw-ui --example gallery && scripts/wayland-smoke.sh
#
# Needs: sway, a cursor theme (Debian/Ubuntu: dmz-cursor-theme; set RECLAW_CURSOR_THEME for another), python3, cargo.
# How it works: sway has no input device when headless, so tools/virtual-pointer gives its seat a pointer. The app runs
# with WAYLAND_DEBUG=1 and the cursor shape it asks for is read from the protocol log (wp_cursor_shape_v1 set_shape).
#
# What it cannot tell you: anything about GNOME's compositor (Mutter), transparency and rounded corners, or a real panel.
set -u
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${RECLAW_BIN:-$ROOT/target/debug/examples/gallery}"
SCALES="${RECLAW_SCALES:-1 1.5 2}"
WORK="$(mktemp -d)"
export XDG_RUNTIME_DIR="$WORK/run"; mkdir -m 700 "$XDG_RUNTIME_DIR"
export WLR_BACKENDS=headless WLR_LIBINPUT_NO_DEVICES=1 WLR_RENDERER=pixman
export XCURSOR_THEME="${RECLAW_CURSOR_THEME:-DMZ-White}" XCURSOR_SIZE=24
export RECLAW_HOME="$WORK/home"
CMDS="$WORK/pointer.cmds"; LOG="$WORK/app.log"; : > "$CMDS"

failures=0
check() { # check <description> <expected> <actual>
  if [ "$2" = "$3" ]; then echo "ok    $1"; else echo "FAIL  $1: expected '$2', got '$3'"; failures=$((failures + 1)); fi
}
cleanup() { kill "${APP:-}" "${SWAY:-}" "${TAIL:-}" "${VPTR:-}" 2>/dev/null; rm -rf "$WORK"; }
trap cleanup EXIT

[ -x "$BIN" ] || { echo "no binary at $BIN; build it first"; exit 2; }
command -v sway >/dev/null || { echo "sway is not installed"; exit 2; }
cargo build --release --quiet --manifest-path "$ROOT/tools/virtual-pointer/Cargo.toml" --target-dir "${RECLAW_VPTR_TARGET:-$WORK/vptr-target}" \
  || { echo "could not build tools/virtual-pointer"; exit 2; }
VPTR_BIN="${RECLAW_VPTR_TARGET:-$WORK/vptr-target}/release/reclaw-virtual-pointer"

cat > "$WORK/sway.conf" <<CONF
xwayland disable
output HEADLESS-1 resolution 1600x1000
default_border none
default_floating_border none
for_window [app_id="dev.reclaw.Reclaw"] floating enable, move position 40 40
seat seat0 xcursor_theme $XCURSOR_THEME 24
CONF
sway -c "$WORK/sway.conf" >"$WORK/sway.log" 2>&1 & SWAY=$!
for _ in $(seq 1 40); do ls "$XDG_RUNTIME_DIR"/wayland-[0-9] >/dev/null 2>&1 && ls "$XDG_RUNTIME_DIR"/sway-ipc.* >/dev/null 2>&1 && break; sleep 0.5; done
WAYLAND_DISPLAY="$(basename "$(ls "$XDG_RUNTIME_DIR"/wayland-[0-9] | head -1)")"; export WAYLAND_DISPLAY
SWAYSOCK="$(ls "$XDG_RUNTIME_DIR"/sway-ipc.* | head -1)"; export SWAYSOCK
[ -n "$WAYLAND_DISPLAY" ] || { echo "sway did not start; see $WORK/sway.log"; exit 2; }

# The pointer first, so the seat has one when the app binds it.
(tail -n0 -f "$CMDS" | "$VPTR_BIN") >"$WORK/vptr.log" 2>&1 & TAIL=$!
sleep 2
env -u DISPLAY WINIT_UNIX_BACKEND=wayland WAYLAND_DEBUG=1 "$BIN" >"$LOG" 2>&1 & APP=$!

rect() { swaymsg -t get_tree | python3 -c "
import sys, json
def walk(n):
    if n.get('app_id') == 'dev.reclaw.Reclaw':
        r = n['rect']; print(r['x'], r['y'], r['width'], r['height'])
    for c in n.get('nodes', []) + n.get('floating_nodes', []): walk(c)
walk(json.load(sys.stdin))"; }
out_size() { swaymsg -t get_outputs | python3 -c "import sys, json; r = json.load(sys.stdin)[0]['rect']; print(r['width'], r['height'])"; }

for _ in $(seq 1 60); do [ -n "$(rect)" ] && break; sleep 0.5; done
[ -n "$(rect)" ] || { echo "the window never appeared; see $LOG"; exit 2; }
sleep 8   # the first frames

# Pointer positions are logical pixels of the output; the virtual pointer takes them as a fraction of 1600x1000.
move() { echo "move $(( $1 * 1600 / OW )) $(( $2 * 1000 / OH ))" >> "$CMDS"; sleep 0.6; }
# The last cursor shape requested after the pointer arrives at ($1, $2), coming from the middle of the window.
shape_at() {
  move $(( WX + WW / 2 )) $(( WY + WH / 2 )); move $(( WX + WW / 2 + 3 )) $(( WY + WH / 2 ))
  local before; before=$(wc -l < "$LOG")
  move "$1" "$2"; move "$1" "$2"; sleep 0.4
  tail -n +$((before + 1)) "$LOG" | grep -o 'set_shape([0-9]*, [0-9]*)' | tail -1 | sed 's/.*, \([0-9]*\))/\1/'
}
# Press at ($1, $2), move by ($3, $4) in four steps, release.
drag() {
  move "$1" "$2"; echo down >> "$CMDS"; sleep 0.5
  for i in 1 2 3 4; do move $(( $1 + $3 * i / 4 )) $(( $2 + $4 * i / 4 )); done
  echo up >> "$CMDS"; sleep 1.5
}

for scale in $SCALES; do
  swaymsg "output HEADLESS-1 scale $scale" >/dev/null; sleep 3
  read -r OW OH < <(out_size)
  swaymsg "resize set width $(( OW - 120 )) px height $(( OH - 100 )) px" >/dev/null; swaymsg "move position 40 40" >/dev/null; sleep 3
  read -r WX WY WW WH < <(rect)
  echo "-- scale $scale: output ${OW}x${OH}, window ${WW}x${WH} (logical)"
  # 26 east-west, 27 north-south, 28 northeast-southwest, 29 northwest-southeast (wp_cursor_shape_v1)
  check "scale $scale: right edge"        26 "$(shape_at $(( WX + WW - 3 )) $(( WY + WH / 2 )))"
  check "scale $scale: left edge"         26 "$(shape_at $(( WX + 3 )) $(( WY + WH / 2 )))"
  check "scale $scale: bottom edge"       27 "$(shape_at $(( WX + WW / 2 )) $(( WY + WH - 3 )))"
  check "scale $scale: top edge"          27 "$(shape_at $(( WX + WW / 2 )) $(( WY + 3 )))"
  check "scale $scale: bottom-right"      29 "$(shape_at $(( WX + WW - 3 )) $(( WY + WH - 3 )))"
  check "scale $scale: top-left"          29 "$(shape_at $(( WX + 3 )) $(( WY + 3 )))"
  check "scale $scale: top-right"         28 "$(shape_at $(( WX + WW - 3 )) $(( WY + 3 )))"
  check "scale $scale: bottom-left"       28 "$(shape_at $(( WX + 3 )) $(( WY + WH - 3 )))"
  inside=$(shape_at $(( WX + WW - 20 )) $(( WY + WH / 2 )))
  case "$inside" in 26|27|28|29) check "scale $scale: 20px in is not a resize edge" "not a resize" "shape $inside" ;; *) echo "ok    scale $scale: 20px in is not a resize edge" ;; esac

  grew() { # grew <description> <axis: 3 = width, 4 = height> <before> <after> <wanted>
    local d=$(( $4 - $3 )); if [ "$d" -ge $(( $5 - 12 )) ] && [ "$d" -le $(( $5 + 12 )) ]; then echo "ok    $1 (${d}px)"; else echo "FAIL  $1: wanted about $5px, got ${d}px"; failures=$((failures + 1)); fi; }
  drag $(( WX + WW - 3 )) $(( WY + WH / 2 )) 60 0; read -r _ _ NW NH < <(rect); grew "scale $scale: dragging the right edge 60px widens it" w "$WW" "$NW" 60
  drag $(( WX + NW / 2 )) $(( WY + NH - 3 )) 0 40;  read -r _ _ MW MH < <(rect); grew "scale $scale: dragging the bottom edge 40px heightens it" h "$NH" "$MH" 40
done
echo
if [ "$failures" -eq 0 ]; then echo "all checks passed"; else echo "$failures check(s) failed"; fi
[ "$failures" -eq 0 ]
