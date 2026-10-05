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

## Updating

| Log line | Meaning | What to do |
|---|---|---|
| `INFO reclaw_app::update: ==> ...` | The update script's own output, line by line. | Read them in order; the last one starts `RECLAW_UPDATE:`. |
| notice "The update stopped: you have local changes" | A tracked file was edited in the checkout. Nothing was changed. | Commit or `git stash`, then update again. |
| notice "The update stopped: the histories differ" | The checkout has commits the remote does not, and the remote has new ones. Nothing was changed. | `git log --oneline --graph --all`; decide how to combine them yourself. |
| notice "The build failed" | The new source is in place; the compiler said no. The last lines of its output are in the notice and the log. | Fix the error (or report it) and update again. |

If the line you have is not here, send the log or the report and this page gets a row for it.
