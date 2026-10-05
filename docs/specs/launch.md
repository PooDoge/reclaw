# Launching

- last-verified: 2026-10-05
- owner-paths: reclaw-runtime/**, reclaw-app/src/host/launch/**, reclaw-ui/src/launch_request.rs, reclaw-ui/src/launch.rs

What Play, Stop and Resume do. The process work is the supervisor's; the choices of what to start and how are the host's. Rules are
Quiver's (`GameLaunchService`, `WindowsRunnerService`, `HostProcessEnvironment`; read, not run) unless the text says otherwise.

## Play

1. The app's folder is the library entry's `installPath`, else its name in the default location. If it is not there the notice says
   where Reclaw looked.
2. The program is the one named in the folder's `selected_executable.txt` (Quiver's file) if it is inside the folder and exists;
   otherwise the nearest to the top (a top-level search first, then the whole tree). Which files count as programs is in the install
   spec. When several qualify and none is chosen Quiver asks; Reclaw takes the nearest and logs the choice (a chooser is not built).
3. **On Linux a Windows `.exe` runs through a runner**: `auto` tries the global custom command, then Proton, then Wine; `proton`,
   `wine` and `custom` force one. The library entry's `linuxRunner`, `linuxPrefixPath`, `linuxProtonPath` and
   `linuxCustomLaunchCommand` are honoured (there is no screen to set them yet). Proton is found in Steam's folders
   (`steamapps/common/Proton*`, `compatibilitytools.d/*Proton*`, including the Flatpak Steam), newest numbered release first and
   *Experimental* last (Quiver orders by name as text, which puts `Proton 9` after `Proton 10`). Prefixes live in the app's folder
   (`.steam-compat-data`, `.wine-prefix`) and are never searched for programs. A custom command may use `{exe}`, `{gamePath}` and
   `{exeDir}`; the program is appended when it names none. With no runner the notice says what to install.
4. A native program is made executable (a zip may have lost its mode).
5. The person's launch settings are applied: the game's declared capabilities and the person's choices become arguments, variables and
   edits to the game's own config files (`reclaw_games::settings::plan`), then the "Launch options" row is split into words
   (quotes respected; a quote left open is reported and the options are not used) and appended. A config edit that fails is
   reported and the game still starts.
6. The environment is cleaned: `LD_LIBRARY_PATH`, `LD_PRELOAD`, the Qt plugin paths, `APPDIR`, `APPIMAGE`, `APPIMAGE_*`, `ARGV0`, `OWD`
   are removed (unless the runner sets them itself); AppImage mount points are taken out of `PATH`; `PWD` is the working directory.
7. The process starts in its own process group; its output is appended to `<logs>/games/<folder name>.log` (restarted when over 5 MiB).
   The card shows *Starting*, then *Running* with the process id when the supervisor reports it started.

## How it ends

The supervisor reports the end and how: exit 0 is *Idle*; a non-zero exit or a signal is *Failed* (the button says Retry); a Stop
the person asked for is not a failure. A notice is shown only for a crash (a signal; the signal is named, SIGSEGV as a crash) and for
an exit within five seconds with a non-zero code ("closed right after starting"), each with where the output is. A non-zero exit
later is how many games quit and is not announced.

## Stop and Resume

Stop asks the app to quit (SIGTERM to the group) and escalates to a kill after eight seconds; pressed again it kills at once. Resume
does nothing in the host: the app already has the screen, and the shell has given it the pad.

## Not built / not verified

* **Run only against shell scripts standing in for games, and a fake `wine`.** No real recompiled game, no real Proton, no real Wine
  from this build environment. Gamepad-driven launching on a Steam Deck, Wayland/GNOME focus behaviour and Windows/macOS paths are
  unverified.
* macOS (`open` on a `.app`) and Windows launching are written in outline and not run.
* No Flatpak apps; no last-played time; no per-launch debug report beyond the log line (program, arguments, working directory, the
  variables removed, the output file).
* A game that starts a launcher process and exits at once looks like it ended; Quiver has the same limit.

## Tests

`reclaw-runtime` (34): the clean environment, runner choice and command building with folders made for the test, shell-like
splitting, Proton ordering. `reclaw-app` host tests with real processes: arguments in order, cwd, output kept, a quick failure, a
crash, a quiet exit, Stop, a double press, missing folder, empty folder, a chosen program and one outside the folder, no runner, a
fake Wine, config edits applied and failing, launch options not used. `reclaw-ui`: the request built from the state.
