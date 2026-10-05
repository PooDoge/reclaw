#!/usr/bin/env bash
# Build Reclaw on Bazzite (or any Fedora Atomic desktop) without touching the immutable host:
# the build happens inside a distrobox container, and the result is an ordinary binary that runs on
# the host, because the container is the same Fedora release as the host.
#
#   scripts/bazzite-build.sh            # build (creates the container the first time)
#   scripts/bazzite-build.sh --install  # also copy the binary and the launcher entry into your home
#
# The build uses --locked, so it compiles exactly the dependency set Cargo.lock names (Freya 0.5.0-rc.8) that the tests ran on.
# Nothing here has been run on Bazzite by the author; see docs/BUILDING.md for what is and is not verified.
set -euo pipefail

NAME="${RECLAW_BUILD_BOX:-reclaw-build}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RELEASE="$(. /etc/os-release && echo "${VERSION_ID:-}")"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target-bazzite}"

command -v distrobox >/dev/null || { echo "distrobox is not installed. Bazzite ships it; on other systems install it first."; exit 2; }
[ -n "$RELEASE" ] || { echo "cannot read the Fedora release from /etc/os-release"; exit 2; }

# Packages the build needs. libudev is for the gamepad reader; the GL/EGL/Wayland/xkbcommon ones are what the
# window and Skia link against; clang and lld are for linking; fontconfig/freetype are for text.
PACKAGES=(gcc gcc-c++ clang lld make pkgconf-pkg-config git curl
  systemd-devel
  mesa-libEGL-devel mesa-libGL-devel mesa-libGLES-devel
  wayland-devel libxkbcommon-devel libxkbcommon-x11-devel
  libX11-devel libXcursor-devel libXi-devel libXrandr-devel
  fontconfig-devel freetype-devel openssl-devel)

if ! distrobox list 2>/dev/null | grep -q " $NAME "; then
  echo "creating the build container $NAME (Fedora $RELEASE) ..."
  distrobox create --yes --name "$NAME" --image "registry.fedoraproject.org/fedora-toolbox:$RELEASE"
fi

# Inside the container: dev packages, Rust (rustup, in the container's home view of yours), then the build.
distrobox enter "$NAME" -- bash -lc "
  set -euo pipefail
  sudo dnf install -y --setopt=install_weak_deps=False ${PACKAGES[*]}
  if ! command -v cargo >/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  fi
  . \"\$HOME/.cargo/env\"
  cd '$ROOT'
  CARGO_TARGET_DIR='$TARGET_DIR' cargo build --locked --release -p reclaw --features gamepad
"

[ -x "$TARGET_DIR/release/reclaw" ] || { echo "the build finished but $TARGET_DIR/release/reclaw is missing"; exit 1; }
echo "built: $TARGET_DIR/release/reclaw"

if [ "${1:-}" = "--install" ]; then
  install -Dm755 "$TARGET_DIR/release/reclaw" "$HOME/.local/bin/reclaw"
  install -Dm644 "$ROOT/packaging/dev.reclaw.Reclaw.desktop" "$HOME/.local/share/applications/dev.reclaw.Reclaw.desktop"
  update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
  echo "installed ~/.local/bin/reclaw and the applications menu entry (log out and in if GNOME does not show it)"
fi
