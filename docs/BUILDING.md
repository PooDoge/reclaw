# Building and running Reclaw

## What there is to run today

There is no installer-and-launcher binary yet: the install and launch backends are not written. What runs is the
whole UI over sample data, in two example programs:

| Program | What it is |
|---|---|
| `gallery` | Both interfaces over sample data, with no gamepad and no processes. For looking at layouts. |
| `deck` | The same shell wired to a real gamepad reader and the process supervisor. "Games" are a shell loop that stands in for a recompiled game, so Play, Resume, Stop and Force quit are real processes. **Update and Install play a scripted download** (about eleven seconds: a progress bar, a build, then finished), so the card progress, the sidebar's Updates section and the notification toast can be tried by hand. |

```sh
cargo run -p reclaw-ui --example gallery
cargo run -p reclaw-ui --example deck --features gamepad
```

`gallery --open /game/1` starts on a page (any path of the router; `/settings`, `/catalog`, `/mods` ...). **F10** switches between the
desktop and Deck mode, **F9** shows a simulated on-screen keyboard.

The sample games' pictures use a `catalog://` address that is never fetched, so by default every picture is a placeholder and the
README on a game page says it could not load. **`RECLAW_LIVE_SAMPLE=1`** points two sample games at real public repositories
(Starfall 64 at Zelda64Recomp, Kart Ruins at Freya): real artwork, a real screenshot and two real READMEs, fetched, cached under
`RECLAW_HOME/cache/media` and drawn. It needs the internet, and `SSL_CERT_FILE` if your network re-signs HTTPS. Settings, favorites and
the window's size and place are saved like the real app's; `RECLAW_HOME=/tmp/reclaw-try` keeps them (and the downloaded
artwork cache) out of your real profile.

## Environment variables

| Variable | Effect |
|---|---|
| `RECLAW_HOME` | Put the config, data and cache folders under one root. |
| `RECLAW_MODE=deck\|desktop` | Start in an interface (otherwise it is detected: SteamOS, gamescope and Steam variables mean Deck). |
| `RECLAW_WINDOW_FRAME=native` | Use the window manager's border and title bar instead of Reclaw's own. The fallback if a compositor mishandles transparent, undecorated windows. |
| `RECLAW_LAYOUT`, `RECLAW_DENSITY`, `RECLAW_THEME`, `RECLAW_MOTION`, `RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD` | Push any build into any form factor, theme or motion setting without a device. See `reclaw-ui/src/shell/overrides.rs`. |
| `SSL_CERT_FILE` | A PEM bundle of extra certificate authorities to trust when downloading artwork and READMEs (a network that re-signs HTTPS needs it). |
| `FREYA_RENDERER=software\|opengl\|vulkan` | Force a graphics backend (the toolkit's own variable). `software` is the first thing to try if the window is black. |
| `WINIT_UNIX_BACKEND=x11` | Run through XWayland instead of native Wayland (the windowing library's variable). |

## Bazzite (Fedora Atomic, GNOME on Wayland)

Bazzite's root is immutable, so Reclaw is built inside a **distrobox** container of the same Fedora release as the host. The binary
it produces runs on the host.

```sh
scripts/bazzite-build.sh              # builds target-bazzite/release/examples/{deck,gallery}
scripts/bazzite-build.sh --install    # also installs ~/.local/bin/reclaw-deck, reclaw-gallery and the launcher entry
```

The script creates the container `reclaw-build` the first time, installs the development packages (`systemd-devel` for the
gamepad reader's libudev, the Mesa EGL/GL/GLES headers, Wayland and xkbcommon, fontconfig, freetype), installs Rust with rustup
if the container has none, and builds in release mode with `--locked`, so it compiles exactly the dependencies in `Cargo.lock`
(Freya is pinned to `=0.5.0-rc.8`; `docs/` and `AGENTS.md` say how that release is the reference). Allow 10 GB of disk for the build: the graphics library (Skia) is large.

Running, to try in this order if something is wrong:

1. `reclaw-deck` (or `cargo run`). The window is Reclaw's own: a title bar with minimize, maximize and close, resize bands
   along the edges, rounded corners.
2. A black or missing window: `FREYA_RENDERER=software reclaw-deck`.
3. GNOME shows no usable window, or corners are black: `RECLAW_WINDOW_FRAME=native reclaw-deck`.
4. Still nothing, or input is odd: `WINIT_UNIX_BACKEND=x11 reclaw-deck`.

GNOME's compositor leaves window decorations to the application (client-side decorations), so a window that asks for none has none, which is why Reclaw draws its own.
The window's app id is `dev.reclaw.Reclaw`; GNOME groups windows and picks the icon by the launcher file of the same name
(`packaging/dev.reclaw.Reclaw.desktop`).

**Controllers.** The gamepad reader (the `gilrs` library) uses evdev and udev, so the logged-in user needs access to the controller's device, which desktop Linux normally grants. If Steam is running it can claim a pad for
Steam Input; start Reclaw with Steam closed, or add Reclaw to Steam as a non-Steam game, where Steam's Guide button is left to
Steam (`SteamGameId` is detected and the Guide button is unbound).

**Bazzite's Steam Deck images.** Gaming mode runs gamescope, a single fullscreen compositor: Reclaw detects it (`GAMESCOPE_WAYLAND_DISPLAY`,
`XDG_CURRENT_DESKTOP=gamescope`), starts in Deck mode, and offers no monitor or window-mode choices for games, because gamescope owns them.

### A first run, in order

Start with `RECLAW_HOME=/tmp/reclaw-try RECLAW_LIVE_SAMPLE=1 reclaw-deck` (an empty profile each time, real artwork on two games).

| Look at | What right looks like | Verified elsewhere? |
|---|---|---|
| The window | A title bar with minimize, maximize and close; drag it to move; double-press it to maximize; resize from every edge and corner; rounded corners with nothing black behind them | X11 only |
| Library | A sidebar with an *Updates* section (three rows with icons), system badges (N64, PS2, GBA), the *Recent* and *Deck mode* buttons, downloads below the hero | X11 |
| A game page | Select Starfall 64, press *Details*: a real hero picture and screenshot, then About, requirements, and further down the real README (collapsed; *Show the whole README*) | X11, over the real network |
| Mouse back/forward buttons, Alt+Left/Right | Move through pages like a browser; *Recent* lists the last pages | headless tests |
| Scripted update | Select Skyward Quest, press *Update*. The Updates row and the progress follow; when it finishes a notice is made | X11 |
| Deck mode (**F10**) | The window fills the screen with no title bar; cards show icon chips and progress bars; **F10** again gives the window back | X11 |
| The toast | After the update finishes, bottom right: *Hold for details* (X) and *Hold to dismiss all* (Y), with a ring that fills while the key is down. Hold `x` for 0.9 s for the details; hold `y` for 1.2 s to dismiss | X11, keyboard only |
| A gamepad | The same as the keyboard, with the controller's own glyphs on the hints; the toast's X and Y are the pad's | **nothing yet** |
| Scale and monitors | Settings > Screen: UI scale; Deck monitor with two or more screens. At 125% or 150% fractional scaling text stays sharp and pointer targets line up | **nothing yet** |

If a row goes wrong, the terminal output and which variable above got it working are the most useful report.

### Not verified on Bazzite

Nobody has run Reclaw on Bazzite yet. What is known and what is not:

| | |
|---|---|
| Verified (Ubuntu 24.04, X11 under Xvfb with openbox, software rendering) | The build, all tests, and `scripts/x11-smoke.sh`: the custom title bar's buttons, dragging, the resize bands, fullscreen in Deck mode and back, closing, and the window's size and place restored on the next start. |
| Verified on native Wayland (sway, headless, `scripts/wayland-smoke.sh`) | The window opens; the resize cursor appears on every edge and corner and a drag resizes, at scales 1.0, 1.5 and 2.0. |
| Not verified | The Fedora package names above (mapped from the Ubuntu ones that built); the distrobox script; GNOME's compositor (Mutter: dragging and resizing, transparency and rounded corners); more than one monitor, and choosing the monitor Deck mode fills; a high-DPI screen; real gamepad hardware and the hold-to-act timing; SteamOS / gamescope. |

If one of those fails, the terminal output and whichever variable above got it working are the most useful report.

## Debian and Ubuntu (what the tests ran on)

```sh
sudo apt install build-essential pkg-config libudev-dev libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 libfontconfig1-dev
cargo test --workspace --features reclaw-ui/gamepad
```

`libxkbcommon-x11-0` is needed at run time on X11. To exercise the real window without a display, `scripts/x11-smoke.sh` runs the gallery
under Xvfb with a window manager (it needs `xvfb openbox xdotool x11-utils`; see its header).

## Windows and macOS

Not built or run. The code avoids Unix-only paths outside the process supervisor (which is `cfg(unix)`), and the window module has
Windows branches, but none of it has been compiled for those targets.

## Disk space

Every rebuild leaves the old test executables behind, about a gigabyte each. If the disk fills, `cargo clean -p reclaw-ui -p reclaw-media
-p reclaw-games -p reclaw-input -p reclaw-config -p reclaw-runtime` removes Reclaw's own artifacts and keeps the compiled libraries,
so the next build is fast.
