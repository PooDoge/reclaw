# When something goes wrong

Reclaw writes a log of everything that goes wrong (`docs/specs/logging.md`). This page lists the lines and what to do about each.
A row marked † was **seen in a real run** of the program (under a virtual display, against the live catalog); the others are messages the
code writes, exercised by tests but not yet seen in the field, so their wording may still change. To look: **Settings, Diagnostics, Open the log folder**
(or `~/.local/state/reclaw/logs/reclaw.log`; `$RECLAW_HOME/logs` if you set that). To ask for help: **Settings, Diagnostics, Save a
diagnostics report**, and attach the file it names. It contains no tokens.

For more detail, set Diagnostics, Log detail to **Detailed**, or start with `RECLAW_LOG=reclaw_net=debug,warn reclaw`.

## The window

| Log line | Meaning | What to do |
|---|---|---|
| † `WARN freya_winit::drivers: Vulkan initialization failed, falling back: Unable to find a Vulkan driver` | Drawing falls back to OpenGL or software. Seen on a virtual display; on real hardware it means the Vulkan driver is missing or broken. | If the window is slow or black: `FREYA_RENDERER=opengl` or `software`. Install the Mesa Vulkan driver in the distrobox if you want Vulkan. |
| † `WARN winit...x11...randr: XRandR reported that the display's 0mm in size` | The display did not say how big it is, so the scale factor falls back to 1. Seen on virtual displays. | On a real screen, set the scale in the desktop's display settings, or Reclaw's UI scale in Settings. |
| † `WARN winit...x11...xdisplay: error setting XSETTINGS; Xft options won't reload automatically` | A desktop setting could not be watched. Harmless. | Nothing. |
| `WARN reclaw_ui::window::platform: the window system would not start a resize` | The compositor refused a drag-resize from Reclaw's edge bands. | `RECLAW_WINDOW_FRAME=native` uses the system's border instead. |
| `ERROR reclaw::panic: panic: ...` followed by numbered lines | The program crashed. The lines are the stack. | Send the log, or the report. |

## The network

| Log line | Meaning | What to do |
|---|---|---|
| † `WARN reclaw_net::net: the network (a proxy or firewall) refused this host; not asking again for a while host=H` | A proxy or firewall answered the connection request with a refusal. Not something the program can fix, and it does not try to get around it. | Allow `H` in the proxy or firewall, or set `HTTPS_PROXY` / `RECLAW_PROXY`. Seen for `thunderstore.io`, `gamebanana.com` and `cdn2.steamgriddb.com` in the development sandbox. |
| † `ERROR reclaw_sync::catalog: the catalog could not be loaded error=the community catalog index could not be loaded: ...` | The index, the first thing needed, could not be fetched, and there is no saved copy. The notice says "There is no saved copy to show". | Read the `error=` and `hint=` that follow. After one good start the saved copy is shown instead (the notice then says "Showing the saved copy"). |
| `WARN reclaw_net::net: rate limit reached host=api.github.com status=403 wait_s=3100 remaining=0` | GitHub's hourly allowance for anonymous requests is spent. The host is not asked again until `wait_s` have passed. | Settings, Network: paste a GitHub token (needs no permissions). The wait ends the moment a new token is saved. |
| `WARN reclaw_net::net: request failed; trying again host=H attempt=1 of=3 wait_ms=... error=...` | One try failed in a way that may pass (timeout, connection reset, a 5xx). | Nothing yet; if all three fail, a `fetch failed` line follows with the cause. |
| `WARN reclaw_net::net: fetch failed url=H/path error=... hint=...` | A request gave up. `hint=` is the sentence shown to the user. | Do what the hint says. A `Tls` error on a company network: set `SSL_CERT_FILE` to the company's CA bundle. |
| `WARN ... the server answered with a bot-check page instead of the data` | The site wants a real browser. Reclaw says who it is and does not pretend to be one. | Use the site's API with a token, or a mirror. |
| `WARN reclaw_net::download: download interrupted; continuing` | A download's connection dropped; it resumes from the same byte. | Nothing. |
| `WARN ... download failed ... the download is not what was promised` | The file's size or SHA-256 differs from what the catalog said, and was deleted. | Try again; if it repeats, the release was changed after the catalog was written. |

## Tokens

| Log line | Meaning | What to do |
|---|---|---|
| `WARN reclaw_net::net: the service refused the access token; it is not sent any more and the request is repeated without it` | GitHub or GitLab answered 401: the token expired or was revoked. | Make a new one (Settings, Network, Create a token), paste it, Save. |
| `WARN reclaw_app::host::credentials: the service refused the pasted token; it was not kept` | You pasted something the service does not accept. | Copy the whole token; a GitHub token needs no permissions. |
| `WARN reclaw_config::secrets: the tokens file could be read by others; made private` | `secrets.toml` had loose permissions; they were fixed. | Nothing. Check how it got that way if you share the machine. |
| `ERROR ... the tokens file could not be read; it is left alone` | `secrets.toml` exists but cannot be read; Reclaw will not write over it. | Fix its permissions or move it, then restart. |
| † `INFO reclaw_app::host::credentials: token checked provider="GitHub" allowance=14,920 of 15,000 requests left, resets in 7 minutes` | The start-up check. | Nothing: this is what a healthy token looks like. |

## Settings and the library

| Log line | Meaning | What to do |
|---|---|---|
| `ERROR reclaw_ui::bootstrap: settings could not be saved error=...` | `settings.toml` could not be written (permissions, disk full). Changes last until you quit. | Free space or fix permissions on the config folder. |
| `WARN reclaw_ui::notices::log: problem shown to the user title="Your library could not be read"` | `apps.json` is damaged or unreadable. Reclaw left it exactly as it is and will not change it. | The notice names the file; fix or move it, then restart. |
| `WARN reclaw_sync::library: could not back up the library before saving; saving anyway` | The backup copy in `backups/` could not be written; the save still happened. | Check the data folder's space and permissions. |

## Installing and starting apps

None of these has been seen in a real run yet: installs and launches were run against local stand-ins for the services and for games.

| Log line | Meaning | What to do |
|---|---|---|
| `WARN reclaw_app::host::install::jobs: the install location is not usable problem=...` | The location box is not a full path (or `~/...`), cannot be made, or the app's folder name is not one plain name. A notice says which. | Type a full path, or change Settings, Library, Default install location. |
| `INFO ...: several downloads fit; taking the best ranked app=N release=TAG chosen=FILE among=K` | A release had more than one file for this machine; Reclaw took the best match (native before Windows, then by name). There is no chooser yet. | If it picked the wrong one, install that file by hand into the app's folder; a chooser is a planned feature. |
| `INFO reclaw_install::install: installing repo=R tag=T asset=FILE folder=PATH` then `installed` | The normal path: download, SHA-256 check when the release states one, unpack, move over the folder, `version.txt` last. | Nothing. |
| `WARN reclaw_install::install: the install failed repo=R tag=T asset=FILE error=...` | Followed by a notice with the reason and a hint. An update leaves the old version in place. A first install that stopped half way leaves `install-incomplete.txt`, which keeps the folder from counting as installed; installing again removes it once the install finishes. | Do what the hint says; the same install can be started again. |
| `WARN reclaw_install::archive::sink: an archive entry was left out entry=... why=...` or `a link in the archive points outside its folder` | The archive held a path that would land outside the app's folder (`..`, an absolute path, a link out). It was not written. | Nothing, unless the game then fails to start: the release was built badly. |
| `INFO reclaw_install::archive: the zip reader cannot handle this archive; trying a tool from the system` | A zip used a method the built-in reader lacks. `unzip`/`7z`/`bsdtar` is used if present. | Install one of those tools if the next line says none could run. |
| `WARN reclaw_app::host::install::actions: an uninstall failed app=N folder=PATH error=...` / `reclaw_install::remove: an uninstall was refused folder=PATH why=...` | Reclaw deletes only a folder marked as an install (`version.txt`, `install-incomplete.txt` or a program in it), and never a short path, your home folder or a parent of it, or one of its own folders. | Remove the folder yourself if you are sure; the app stays in the library. |
| `WARN reclaw_app::host::launch: an app could not be started app=N error=...` | The program was not found, is not executable, or no runner (Proton or Wine) was found for a Windows `.exe`. The notice says which. | Install Wine, or Steam with Proton; or set a custom runner command in the library entry (`linuxCustomLaunchCommand`). |
| `INFO reclaw_app::host::launch::events: an app started app=N pid=P` / `an app ended app=N outcome=...` | The supervisor's two events for a run. The notice after a failed run names the exit code or signal. | Read the app's own output in `games/<folder name>.log` inside the log folder. |
| `WARN reclaw_app::host::launch: launch settings could not be written to the app's config app=NAME error=...` | The game's own config file could not be edited (permissions, a damaged file). The game starts with the setting it had. | Fix the file's permissions or remove the file so the game writes a fresh one. |

## Updating

| Log line | Meaning | What to do |
|---|---|---|
| `INFO reclaw_app::update: ==> ...` | The update script's own output, line by line. | Read them in order; the last one starts `RECLAW_UPDATE:`. |
| notice "The update stopped: you have local changes" | A tracked file was edited in the checkout. Nothing was changed. | Commit or `git stash`, then update again. |
| notice "The update stopped: the histories differ" | The checkout has commits the remote does not, and the remote has new ones. Nothing was changed. | `git log --oneline --graph --all`; decide how to combine them yourself. |
| notice "The build failed" | The new source is in place; the compiler said no. The last lines of its output are in the notice and the log. | Fix the error (or report it) and update again. |

If the line you have is not here, send the log or the report and this page gets a row for it.
