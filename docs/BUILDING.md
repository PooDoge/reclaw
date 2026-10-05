# Building and running Reclaw

## What there is to run today

```sh
cargo run -p reclaw                       # the launcher: desktop or Deck mode, with the real community catalog
cargo run -p reclaw --features gamepad    # with a real gamepad reader (needs libudev)
```

`reclaw` has no sample data. On start it shows what the last run saved (the community catalog, your library), refreshes the catalog in
the background (about 650 ms on a good connection: the index, four lists and the platform metadata), and shows how fresh it is under
the Catalog's title and in the bottom strip. **Add to library** (Options > Add to library, in the desktop's Manage menu and in Deck's
Options) writes your library file (`apps.json`, Quiver's format). **Install, Play, Update and the rest need the installer, which is not
built yet**: pressing them says so and, for Install, keeps the game in your library. See `docs/quiver-parity.md`.

`reclaw --open /catalog` starts on a page (any path of the router; `/settings`, `/mods`, `/game/<id>` ...). **F10** switches between the
desktop and Deck mode, **F9** shows a simulated on-screen keyboard. Settings, favorites and the window's size and place are saved;
`RECLAW_HOME=/tmp/reclaw-try` keeps all of Reclaw's folders (settings, library, caches) out of your real profile. `RECLAW_CATALOG_INDEX=<url>`
points it at another catalog index, which is how a catalog of your own would be tried.

## Getting the code, and keeping it current

The first time, on the machine that will run it (Bazzite shown; `cargo build --release -p reclaw` replaces the last line elsewhere):

```sh
git clone https://github.com/PooDoge/reclaw && cd reclaw
git checkout claude/game-installer-design-system-m3amsa    # the branch the work is on; main until it is merged
scripts/bazzite-build.sh --install                         # builds in a distrobox, installs ~/.local/bin/reclaw
```

After that, either `scripts/update.sh` in a terminal, or **Settings, About, Update from source** inside Reclaw: both fetch the branch you are
on, fast-forward it, rebuild, and say to quit and start again (`docs/specs/updates.md`). They never reset, stash or discard anything; if
you have edited a tracked file, or your branch has commits the remote does not, they stop and say so. About shows the commit that is
running, so you can tell whether the new build is the one you are looking at.

## When something goes wrong

Reclaw logs everything that fails, to `~/.local/state/reclaw/logs/reclaw.log` (or `$RECLAW_HOME/logs`). **Settings, Diagnostics** opens the
folder, sets how much is logged, and **Save a diagnostics report** writes one file with the versions, the system, what every service
answers from your machine and the end of the log: attach that when you report a problem. `docs/troubleshooting.md` lists the lines.

## GitHub and GitLab tokens

GitHub allows 60 requests an hour to a client without a token and 5,000 with one, and the token needs **no permissions**. **Settings,
Network** has Create a token (opens GitHub with the name filled in), a box to paste it, Save token (checks it with GitHub and shows how many
requests are left), Check token and Remove. The token is kept in `~/.config/reclaw/secrets.toml`, readable only by you, and sent only to
`api.github.com`. `GITHUB_TOKEN` in the environment works too and is used when nothing is saved. GitLab's token is optional (`read_api`).
Nothing in Reclaw calls GitHub's API yet except this check; the token pays off when releases are fetched (`docs/quiver-parity.md`).

## When a download fails

`cargo run -p reclaw-net --example probe` contacts every host the program uses, from your machine, and says for each whether the name
resolved, whether TLS worked, what the server answered, and whether a refusal came from a proxy or firewall, a bot check or a rate
limit (`docs/specs/network.md`). Run it first; it shows what the program sees, which curl may not.

## Environment variables

| Variable | Effect |
|---|---|
| `RECLAW_HOME` | Put the config, data and cache folders under one root. |
| `RECLAW_CATALOG_INDEX` | The address of a community catalog index other than Quiver's. |
| `RECLAW_LOG=<filter>` | The log detail, as `reclaw_net=debug,warn` (a module or crate, a level; the rest default). Replaces the Settings choice. |
| `RECLAW_MODE=deck\|desktop` | Start in an interface (otherwise it is detected: SteamOS, gamescope and Steam variables mean Deck). |
| `RECLAW_WINDOW_FRAME=native` | Use the window manager's border and title bar instead of Reclaw's own. The fallback if a compositor mishandles transparent, undecorated windows. |
| `RECLAW_LAYOUT`, `RECLAW_DENSITY`, `RECLAW_THEME`, `RECLAW_MOTION`, `RECLAW_KEYBOARD`, `RECLAW_SIM_KEYBOARD` | Push any build into any form factor, theme or motion setting without a device. See `reclaw-ui/src/shell/overrides.rs`. |
| `SSL_CERT_FILE` | A PEM bundle of extra certificate authorities to trust for every download (a network that re-signs HTTPS needs it). |
| `HTTPS_PROXY`, `NO_PROXY`, `RECLAW_PROXY=none\|<url>` | The proxy to use; `RECLAW_PROXY` overrides the system's. |
| `GITHUB_TOKEN`, `GITLAB_TOKEN` (or the `RECLAW_` forms) | Raise the services' rate limits (GitHub allows 60 anonymous requests an hour). Sent only to that service's own host. |
| `FREYA_RENDERER=software\|opengl\|vulkan` | Force a graphics backend (the toolkit's own variable). `software` is the first thing to try if the window is black. |
| `WINIT_UNIX_BACKEND=x11` | Run through XWayland instead of native Wayland (the windowing library's variable). |

## Bazzite (Fedora Atomic, GNOME on Wayland)

Bazzite's root is immutable, so Reclaw is built inside a **distrobox** container of the same Fedora release as the host. The binary
it produces runs on the host.

```sh
scripts/bazzite-build.sh              # builds target-bazzite/release/reclaw
scripts/bazzite-build.sh --install    # also installs ~/.local/bin/reclaw and the launcher entry
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

Start with `RECLAW_HOME=/tmp/reclaw-try reclaw` (an empty profile each time; the real catalog is fetched).

| Look at | What right looks like | Verified elsewhere? |
|---|---|---|
| The window | A title bar with minimize, maximize and close; drag it to move; double-press it to maximize; resize from every edge and corner; rounded corners with nothing black behind them | X11 only |
| Catalog | After a second or two the bottom strip reads "232 apps" and "Catalog updated just now"; system chips (N64, PS2 ...) filter; artwork fills in | X11, over the real network |
| Library | Empty, with *Browse the catalog*. In the Catalog, open an app's menu, *Add to library*: it appears in the Library and survives a restart | X11 |
| Install | Open an app, *Install*: the location box already holds Settings > Library > *Default install location* (or `~/Reclaw/Apps`). The Downloads page shows Downloading, Verifying, Extracting, Finishing; the app's card then reads *Installed* with its version, and the folder holds the game's files plus `version.txt` | fake release servers and archives only; **no real recompiled game yet** |
| Game page banner | A wide picture if the catalog or the README has one; otherwise a coloured banner (the same colour every time for a game) with its icon in the middle and the title on the strip below. Never an empty box or a label | fake pictures and a fake README; the generated banner checked in snapshots |
| Play, Stop, Uninstall | *Play* starts the program and the button becomes *Stop*; when the game ends the card says how (a failed run names the exit code or signal and where its output went). *Uninstall* asks first, deletes the folder and leaves the app in the library | shell scripts standing in for games; **no real game, no real Proton or Wine** |
| Settings, Network | GitHub's status reads "No token (60 requests an hour)" (or "From GITHUB_TOKEN"). *Check token* shows a notice with the allowance left. Pasting a token and *Save token* checks it and keeps it only if GitHub accepts it | headless tests; the check against the real GitHub through a proxy |
| Settings, Diagnostics | *Open the log folder* shows `reclaw.log`; *Save a diagnostics report* writes `diagnostics-<time>.txt` and a notice names it | headless tests; the log read from a real run |
| Settings, About | *Build* shows the commit that is running; *Update from source* runs `scripts/update.sh` and ends with a notice | the script against real git repositories; **the button on a desktop: nothing yet** |
| Deck mode (**F10**) | The window fills the screen with no title bar; **F10** again gives the window back | X11 |
| The toast | When a notice appears (try *Check token*), bottom right: *Hold for details* (X) and *Hold to dismiss all* (Y), with a ring that fills while the key is down. Hold `x` for 0.9 s for the details; hold `y` for 1.2 s to dismiss | X11, keyboard only |
| Mouse back/forward buttons, Alt+Left/Right | Move through pages like a browser; *Recent* lists the last pages | headless tests |
| A gamepad | The same as the keyboard, with the controller's own glyphs on the hints | **nothing yet** |
| Scale and monitors | Settings > Screen: UI scale; Deck monitor with two or more screens. At 125% or 150% fractional scaling text stays sharp and pointer targets line up | **nothing yet** |

If a row goes wrong, **Settings, Diagnostics, Save a diagnostics report** and the file it names are the most useful report.

### Not verified on Bazzite

Nobody has run Reclaw on Bazzite yet. What is known and what is not:

| | |
|---|---|
| Verified (Ubuntu 24.04, X11 under Xvfb with openbox, software rendering) | The build, all tests, and `scripts/x11-smoke.sh`: the custom title bar's buttons, dragging, the resize bands, fullscreen in Deck mode and back, closing, and the window's size and place restored on the next start. |
| Verified on native Wayland (sway, headless, `scripts/wayland-smoke.sh`) | The window opens; the resize cursor appears on every edge and corner and a drag resizes, at scales 1.0, 1.5 and 2.0. |
| Verified against the live internet (2026-10-05, the real `reclaw` binary under Xvfb) | The catalog (index, four lists, platform metadata) loaded and was saved; 232 projects drawn with their systems; 56 icons hosted on `raw.githubusercontent.com` fetched and drawn; the `probe` example classified every host. Hosts the build sandbox's proxy refuses (`thunderstore.io`, `gamebanana.com`, `cdn2.steamgriddb.com`) were reported as refused and their pictures stayed placeholders. |
| Verified this session (2026-10-05) | The log: the real binary under Xvfb wrote `reclaw.log` with the version and commit, the detected environment, the token check, the catalog load (232 apps, 1.5 s cold, 35 ms from the saved copy), the window system's warnings (no Vulkan driver, a 0 mm display) and, when pointed at a host the network refuses, the whole failure from the refusal to the notice shown; the sandbox's real token appeared nowhere in the log, settings or cache. Against the live GitHub API (through the sandbox's gateway, which answers for its own account): the allowance check and its fields, the token-expiry header, and that a conditional request answered `304` costs nothing. `scripts/update.sh` against real git repositories: fast-forward, up to date, ahead, diverged, local changes, no upstream, detached head, unreachable remote, build success, build failure. |
| Not verified | The `401` GitHub gives a refused token and `X-OAuth-Scopes` on a real classic token (the sandbox's proxy substitutes its own credential, so a made-up token was "accepted"); anything GitLab does with a token; the Settings > About update button on a desktop and the real build step; the log folder on Windows and macOS; downloading a real release asset and installing a real recompiled game from it (github.com and its release downloads were unreachable from the build sandbox, so the installer ran against local servers and archives); starting a real game, Proton, Wine, an AppImage (needs FUSE) or a game from the Steam Deck's gamepad; the Settings > Library default location on a real desktop; banners from real READMEs; the mod sites' real responses; the Fedora package names above (mapped from the Ubuntu ones that built); the distrobox script; GNOME's compositor (Mutter: dragging and resizing, transparency and rounded corners); more than one monitor, and choosing the monitor Deck mode fills; a high-DPI screen; real gamepad hardware and the hold-to-act timing; SteamOS / gamescope. |

If one of those fails, the terminal output and whichever variable above got it working are the most useful report.

## Debian and Ubuntu (what the tests ran on)

```sh
sudo apt install build-essential pkg-config libudev-dev libegl1-mesa-dev libgl1-mesa-dev libgles2-mesa-dev \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-0 libfontconfig1-dev
cargo test --workspace --features reclaw-ui/gamepad
```

`libxkbcommon-x11-0` is needed at run time on X11. To exercise the real window without a display, `scripts/x11-smoke.sh` runs the launcher
under Xvfb with a window manager (it needs `xvfb openbox xdotool x11-utils`; see its header).

## Windows and macOS

Not built or run. The code avoids Unix-only paths outside the process supervisor (which is `cfg(unix)`), and the window module has
Windows branches, but none of it has been compiled for those targets.

## Disk space

Every rebuild leaves the old test executables behind, about a gigabyte each. If the disk fills, `cargo clean -p reclaw-ui -p reclaw-media
-p reclaw-games -p reclaw-input -p reclaw-config -p reclaw-runtime` removes Reclaw's own artifacts and keeps the compiled libraries,
so the next build is fast.
