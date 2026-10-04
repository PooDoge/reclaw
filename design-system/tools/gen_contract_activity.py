"""Extends reclaw.freya.json with the shared store, routing, background activity, notices, the window frame,
fetched media and systems; refreshes the stale parts (notBuilt, verification, codeMap, the Effect list).
Idempotent: every entry is keyed and replaced. Run from anywhere, AFTER gen_contract_surfaces.py (which sets
notBuilt and verification wholesale): `python3 design-system/tools/gen_contract_activity.py`."""
import os, json
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
P = "reclaw.freya.json"
d = json.load(open(P))

def upsert(items, entry, key):
    for i, x in enumerate(items):
        if x[key] == entry[key]:
            items[i] = entry
            return
    items.append(entry)

def comp(name, freya, rust, **kw):
    e = {"name": name, "freya": freya, "rust": {"type": rust.split("::")[-1], "module": "reclaw_ui::" + "::".join(rust.split("::")[:-1]), "built": "custom"}}
    e.update(kw)
    return e

# ---- data model
dm = d["dataModel"]
dm["Effect"] = ("Launch(id) | Resume(id) | Stop(id) | StartInstall{app,location,game_file,shortcut,prerelease} | Update(id) | ChooseFile(id) | OpenFolder(id) | Verify(id) | "
                "CheckUpdate(id) | Uninstall(id) | ToggleFavorite(id) | AddToCollection{app,name} | NewCollection(id) | CancelActivity(id) | DismissActivity(id) | DismissNotice(id) | "
                "DismissAllNotices | Search | InstallMod{provider,id} | RemoveMod{provider,id} | OpenUrl(url) | SwitchToDesktop | Navigate(Route) | Back | LaunchSetting{app,key,value} | "
                "SetMode(Auto|Desktop|Deck) | Setting(change) | BeginTextEntry(field) | EndTextEntry(field) | TextCommitted{app,field,value} | BringLauncherToFront | SendLauncherToBack | "
                "InputOwner(Launcher|App) | NoticeHolds(bool) | Window(WindowCommand). Defined in reclaw_ui::effect, shared by both interfaces. The Shell handles Window and mode effects itself; "
                "the host handles the rest. SubmitInstall is internal: DeckApp turns it into StartInstall because it holds the form's text.")
dm["HostState"] = "REPLACED by AppState (see `state`). The host writes through Store / StoreFeed; both interfaces read through the use_* hooks."
dm["AppState"] = ("{games, projects, mods, activity: ActivityBoard, notices: Notices, settings: SettingsValues, launch: LaunchPrefs, favorites, window: WindowPrefs, controller, keyboard_inset, "
                  "display: DisplayEnvironment, mailbox, settings_warning}. Plain data, no handles. Changed only by AppAction through reduce(); saved parts via to_prefs/from_prefs.")
dm["Activity"] = "{id, game_id, kind: Install|Update|Mod{provider,id}, title, stage: Queued|Downloading|Verifying|CheckingGameFile|Building|Extracting, bytes_done, bytes_total: Option<u64>, rate: Option<u64>, changelog: Option<Changelog>, outcome: Running|Finished|Failed{reason}|Cancelled}. Progress and time left are derived (activity::format)."
dm["Indicator"] = "{kind: UpdateAvailable|Queued|Downloading|Installing|Done|Failed|Mods, progress: Option<f32>, label: String, mods: usize}. indicator_for(board, game); pure."
dm["Notice"] = "{id, kind: UpdateAvailable|UpdateFinished|InstallFinished|DownloadFailed|ModInstalled, game_id: Option<u32>, title, body, details: Vec<String>, url: Option<String>}. A repeat replaces; the newest is shown."
dm["Route"] = "freya-router enum, the only list of pages: / Library, /catalog, /game/:id, /game/:id/install, /game/:id/settings, /game/:id/settings/:section, /mods, /mods/:provider/:mod_id, /downloads, /settings, /settings/:section. Metadata in nav::meta."
dm["WindowCommand"] = "Minimize | ToggleMaximize | Close | Fullscreen{monitor: Option<usize>} | Windowed"
dm["DisplayEnvironment"] = "{server: Wayland|X11|Gamescope|Windows|MacOs|Unknown, monitors: [Monitor{id, name, native, refresh_mhz, primary}]} from window::monitors::environment; also what launch settings read."
dm["Preferences"] = "reclaw_config: settings by key (choices by position), launch defaults and overrides, favorites, window placement. One TOML file; every field has a default."
dm["Platform"] = "reclaw_games::project::Platform: the system a game was recompiled from (short name, full name, maker, ordering). SystemKind groups them."

# ---- sections
d["state"] = {
    "spec": "docs/specs/state.md", "adr": "docs/adr/0002-shared-state-in-one-store.md",
    "rust": "reclaw_ui::store (AppState, AppAction, reduce, AppChannel, Store, StoreFeed, use_* hooks); reclaw_config (Preferences, PrefsWriter)",
    "kinds": [
        {"kind": "shared", "where": "AppState in one freya-radio station", "for": "anything two windows, both interfaces or a background thread must agree on"},
        {"kind": "window", "where": "use_state / context in that window", "for": "search text, selection, dialog open"},
        {"kind": "fetched", "where": "reclaw-media cache via use_remote_file", "for": "artwork, screenshots, README"},
    ],
    "rules": [
        "AppState changes only through AppAction -> reduce (pure, tested); components never write it",
        "readers subscribe to an AppChannel so a progress tick redraws only what shows progress",
        "other threads send actions through StoreFeed, drained by one UI task",
        "saved: settings by key (choices by position: append options, never reorder), launch settings, favorites, window prefs",
        "an unreadable file is set aside as .bad; a file from a newer Reclaw is read but never overwritten; writes are atomic and debounced, flushed on close",
        "settings are data (a schema both interfaces render): adding a Row adds it everywhere",
    ],
}
d["routing"] = {
    "spec": "docs/specs/routing.md", "adr": "docs/adr/0001-routing-with-freya-router.md",
    "rust": "reclaw_ui::nav (Route, Nav, RouteStage, Recents, transition::*), reclaw_ui::pages",
    "rules": [
        "freya-router is the truth about which page shows; Deck's reducer asks it to move (Effect::Navigate, Effect::Back) and follows when it moves for another reason",
        "roots (tabs) replace, other pages push; Back closes overlays first, then goes back in history, then up",
        "mouse back/forward buttons and Alt+Left/Right do the same as B",
        "transitions have a style and intensity per interface and per page; reduced motion makes them instant; the maths is pure (nav::transition)",
        "Recents is a separate list fed on every change (the router's history cannot be read and has the wrong shape)",
    ],
}
d["activity"] = {
    "spec": "docs/specs/state.md", "rust": "reclaw_ui::activity (ActivityBoard, ActivityEvent, Indicator, indicator_for, format_eta/format_rate)",
    "rules": [
        "the host sends ActivityEvents; the UI never creates or changes an activity",
        "a finished update stays on the board until the app restarts, with what changed",
        "a card shows an icon chip and a bar: fills when the size is known, slides when not, full and green when finished, absent when only an update waits or it failed",
        "the desktop sidebar has an Updates section listing games with an indicator; finished updates keep their row until restart",
    ],
    "indicators": {"UpdateAvailable": ["download", "warn"], "Queued": ["clock", "ink-muted"], "Downloading": ["arrow-down-to-line", "accent"], "Installing": ["package", "info"],
                   "Done": ["check", "ok"], "Failed": ["triangle-alert", "danger"], "Mods": ["puzzle", "accent"]},
}
d["notices"] = {
    "spec": "docs/specs/notices-and-holds.md", "adr": "docs/adr/0006-holds-in-the-pad-reader.md",
    "rust": "reclaw_ui::notices (Notice, Notices, HoldAction); reclaw_input::HoldTracker; reclaw_ui::deck::{NoticeToast, NoticeDetails}",
    "holds": [{"button": "X (west)", "action": "details", "afterMs": 900}, {"button": "Y (north)", "action": "dismiss all", "afterMs": 1200}],
    "rules": [
        "HoldTracker is pure with the clock passed in; the pad reader runs it only while the UI says a toast is up (Effect::NoticeHolds)",
        "a held button acts on release: early release is its ordinary action (X Options, Y Search), a completed hold fires once",
        "the toast is not focusable and never takes the pad from the page",
        "the keyboard X and Y keys use the same tracker, so Deck mode is testable without a pad",
    ],
}
d["window"] = {
    "spec": "docs/specs/window.md", "adr": "docs/adr/0005-custom-window-frame.md",
    "rust": "reclaw_ui::window (WindowCommand, Frame, Titlebar, monitors, geometry, policy, driver, launch)",
    "rules": [
        "no native decoration and a transparent window by default; BorderlessPlugin supplies resize bands and corners; RECLAW_WINDOW_FRAME=native turns it off",
        "the title bar is part of the desktop interface only",
        "every request to the window is an Effect::Window the Shell carries out; components never call winit or Platform",
        "monitors are read from winit once and on scale change (winit 0.30 has no hot-plug event); numbered left to right, then top to bottom",
        "a saved size/position is restored only if it still lands on a connected monitor; on Wayland only the size is used",
        "Deck mode fills the screen (borderless fullscreen on the chosen monitor) and leaves fullscreen only if Reclaw made it so",
    ],
    "settings": ["KEY_UI_SCALE", "KEY_DECK_FULLSCREEN", "KEY_DECK_DISPLAY"],
}
d["media"] = {
    "spec": "docs/specs/media.md", "adr": ["docs/adr/0003-media-through-one-crate.md", "docs/adr/0004-readme-on-the-stock-markdown-viewer.md"],
    "rust": "reclaw_media (policy, sniff, store, cache, hub, readme); reclaw_ui::media::use_remote_file; reclaw_ui::readme (ReadmeSection, ReadmeView, FitPicture, BadgeChip)",
    "rules": [
        "only reclaw-media fetches: https only, standard port, no credentials, nothing on the local network (rechecked after redirects), size and time caps",
        "the file kind is decided from the bytes, not the server's Content-Type",
        "the cache is on disk, atomic, trimmed to a size, with freshness, a stale fallback and failure memory; the Settings switch turns downloads off",
        "the toolkit's remote-asset and html features are off; markdown is cut into blocks before the stock MarkdownViewer sees it; images in prose become links",
        "README: pictures, badge rows and diagrams are blocks drawn by our components; script, style, iframe, svg and form are dropped with their contents",
    ],
    "freyaHtml": "evaluated by reading its source and not adopted: it fetches subresources with no policy, navigates inside the viewer on a link, reports no height, and pulls in Blitz and Stylo",
}
d["systems"] = {
    "spec": "docs/specs/systems.md", "rust": "reclaw_games::project::Platform; reclaw_ui::systems; SystemBadge",
    "rules": [
        "one table (Platform) names, orders and groups the systems; Other draws no badge",
        "the Library filters by system with a count per system; the Catalog's system chips are in system order; Deck has no system filter, it groups instead",
        "one saved setting, Library > Sort games by, orders the Library, the Catalog and Deck: Added (default), Title or System",
        "Deck Home groups its shelves by system when sorted by System",
    ],
}

# ---- components
upsert(d["components"], comp("SystemBadge", "rect + text", "components::SystemBadge", tokens=["line-strong", "ink-muted", "ink", "bg-base", "radius-sm"],
    props={"platform": "Platform", "over_art": "bool", "large": "bool"},
    note="Neutral chip with the system's short name; draws nothing for an unknown system. over_art gives a solid fill; large is the Deck size. Preview: components/SystemBadge."), "name")
upsert(d["components"], comp("BadgeChip", "rect + text", "readme::BadgeChip", tokens=["bg-raised", "ink-muted"],
    props={"badge": "Badge{label,message,color}", "link": "Option<String>", "on_open": "EventHandler<String>"},
    note="A README status badge drawn natively because the toolkit's SVG drawing has no fonts. Preview: components/BadgeChip."), "name")
upsert(d["components"], comp("IndicatorBadge", "rect + icon + text", "components::IndicatorBadge", tokens=["warn", "accent", "info", "ok", "danger", "ink-muted"],
    props={"indicator": "Indicator", "large": "bool", "filled": "bool"},
    note="Icon and short text for one game's background work. Table in `activity.indicators`. Preview: components/IndicatorBadge."), "name")
upsert(d["components"], comp("CardIndicator", "rect + canvas-free bar", "deck::CardIndicator", tokens=["bg-base", "bg-raised", "accent", "info", "ok", "ink-muted"],
    props={"indicator": "Indicator"}, size={"strip": "8px (CARD_STRIP_H, a Rust constant)", "card": "deck-tile-w x deck-tile-h"},
    note="Chip at the top left of a Deck card and a bar along its bottom edge; the bar slides while the size is unknown. Reduced motion for the sliding bar is not implemented. Preview: components/CardIndicator."), "name")
upsert(d["components"], comp("NoticeToast", "rect + canvas ring", "deck::NoticeToast", tokens=["bg-panel", "warn", "ok", "danger", "accent", "line-strong", "ink", "ink-muted", "radius-lg"],
    props={"notice": "Notice", "holding": "Option<Button>", "kind": "ControllerKind", "last_input": "LastInput", "window": "(f32,f32)"},
    size={"toast": "460x132 (TOAST_W, TOAST_H: Rust constants, not tokens)", "ring": "44px, 4px stroke"},
    note="Newest notice at the bottom right with hold-X and hold-Y prompts; the ring (Skia arc) sweeps clockwise while held. Not focusable. See `notices`. Preview: components/NoticeToast."), "name")
upsert(d["components"], comp("Titlebar", "rect + window_drag", "window::Titlebar", tokens=["bg-deep", "bg-raised", "ink", "ink-muted", "danger", "on-accent"],
    props={"on_command": "EventHandler<WindowCommand>", "attached": "bool", "page": "&'static str"}, size={"height": "36px (TITLEBAR_H, a Rust constant)", "button": "46px wide"},
    note="Our own title bar: drag area, minimize, maximize or restore, close. Desktop only. Verified on X11, not on Wayland. Preview: components/Titlebar."), "name")
upsert(d["components"], comp("RemoteArt", "ImageViewer | SvgViewer + ArtPlaceholder", "components::RemoteArt", tokens=["bg-raised", "bg-panel", "ink-subtle"],
    props={"url": "Option<String>", "placeholder": "ArtPlaceholder", "width": "Size", "height": "Size"},
    note="A fetched picture in the place of a placeholder; the placeholder stays if the file never arrives. Never fetches by itself (see `media`). Preview: components/RemoteArt."), "name")

# ---- decisions
for e in [
    {"id": "one-router", "decision": "freya-router is the one truth about which page shows, for both interfaces.", "why": "Two navigation stacks drift; the mouse's back button moved one and not the other.",
     "rejected": ["a second stack in Deck's reducer", "nested layouts per section", "our own route enum with no router", "reading history back for Recents"], "adr": "0001"},
    {"id": "one-store", "decision": "Shared state is one AppState in a freya-radio station, changed only by AppAction; the saved part is a small TOML file.", "why": "Windows and modes must agree on the library, activity and settings.",
     "rejected": ["context or use_state per window", "freya-query for the app's own state", "one signal with no channels", "SQLite or JSON for settings", "saving the whole AppState"], "adr": "0002"},
    {"id": "media-one-crate", "decision": "Only reclaw-media fetches from the internet; the toolkit's remote-asset and html features are off.", "why": "README images are chosen by strangers; the toolkit has no size limit, address policy or disk cache.",
     "rejected": ["the toolkit's remote images", "fetching in the UI crate", "trusting Content-Type", "DefaultHasher cache names", "no disk cache"], "adr": "0003"},
    {"id": "markdown-stock", "decision": "READMEs are cut into blocks before the stock markdown viewer; no fork and no HTML engine.", "why": "A fork is rebased on every Freya release; what READMEs need is outside the viewer.",
     "rejected": ["forking freya-markdown", "freya-html (Blitz)", "a line-based fence splitter", "badges from their SVG"], "adr": "0004"},
    {"id": "own-window-frame", "decision": "Reclaw draws its own title bar on a borderless transparent window; RECLAW_WINDOW_FRAME=native is the escape hatch.", "why": "GNOME leaves decoration to the application; one look on every desktop.",
     "rejected": ["native decorations everywhere", "a custom bar inside native decoration", "calling winit from components", "the toolkit's use_maximized hook", "the toolkit's close call"], "adr": "0005"},
    {"id": "holds-in-reader", "decision": "Press-and-hold is decided in the pad reader (pure HoldTracker); a held button acts on release while a toast is up.", "why": "A tap must keep its ordinary meaning.",
     "rejected": ["timers in the UI", "dedicated buttons", "acting on press and cancelling", "holds always on"], "adr": "0006"},
]:
    upsert(d["decisions"], e, "id")

# ---- refreshed parts
d["notBuilt"] = [
    "OnScreenKeyboard (the OS provides one; the store's keyboard_inset is how the UI learns its height; a simulated one is built into Shell for testing)",
    "FileBrowser (gamepad file picker); Effect::ChooseFile asks the host and the host answers through the store's mailbox",
    "Deck's Catalog and Mods pages and the README on Deck (the desktop has all of them)",
    "A production host binary: only the gallery and deck examples start the app; the install backend (downloads, builds) is not here, the store accepts its ActivityEvents",
    "Mermaid diagrams in READMEs (shown as source; the parsers in oxide-code are Freya-free and could be ported)",
    "README strikethrough, task-list boxes and footnotes (shown as typed); selecting and copying README text",
    "A cache size and a Clear cache button in Settings",
    "Reduced motion for the sliding card bar",
    "Monitor hot-plug (winit 0.30 has no event; the list refreshes on scale change)",
    "A caret-aware text box (Freya 0.5-rc Input cannot be told where its caret is)",
    "Detecting touch input to switch the desktop to Touch density automatically (today: RECLAW_DENSITY or the host sets it)",
    "Deck mode below about 520px wide is tolerable but not designed (tab strip text wraps)",
    "Virtual keyboard/mouse device (uinput) for apps that do not read SDL",
    "Bringing the launcher/app window forward (platform glue; the model emits Effects)",
    "Windows Guide-button capture and Windows job-object process control (taskkill fallback is untested)",
]
v = d["verification"]
v["workspace"] = ("cargo test --workspace: reclaw-input, reclaw-runtime, reclaw-games, reclaw-config, reclaw-media and reclaw-ui (unit tests beside the code; UI integration tests are one binary, "
                  "reclaw-ui/tests/ui, listed in its main.rs; plus tests/repo_hygiene.rs). ARCHITECTURE.md has the map.")
v["driftGuard"] = "reclaw-ui/tests/ui/tokens_in_sync.rs fails if any color token (both themes), spacing, radius, layout or deck type size in tokens.json differs from the Rust constants."
v["hygiene"] = "tests/repo_hygiene.rs: no Rust file over 1000 lines, every module root documented, no file with more than 150 lines of inline tests, every spec has last-verified and owner paths that match files, ADR names and status."
v["windowFrame"] = "scripts/x11-smoke.sh opens a real window under Xvfb + openbox: size, drag, resize bands, maximize, close. tests/ui/window_chrome.rs checks the title bar emits the right commands."
v["media"] = "reclaw-media has a fake network (hostile inputs, redirects to local addresses, oversize, wrong content types) and an ignored real-network test (cargo test -p reclaw-media --test real_network -- --ignored)."
v["endToEnd"] = ("tests/ui/deck_lifecycle.rs drives the real DeckApp with key presses against a real process: Play launches it and hands the pad to the app (InputOwner(App)); Guide brings the launcher forward; "
                 "Back sends it back; Stop ends the process group; the app ending brings the launcher forward again.")
v["notVerified"] = list(dict.fromkeys(v["notVerified"] + [
    "Wayland/GNOME on Bazzite: the custom frame, transparency, monitor list and Deck fullscreen (only X11 under Xvfb was run)",
    "the hold durations (900 ms, 1200 ms) on a real controller",
    "a build on Bazzite itself (scripts/bazzite-build.sh and docs/BUILDING.md were written and shell-checked, not run there)",
    "HiDPI with a fractional scale on a real display",
    "a README fetch through a network that re-signs HTTPS (SSL_CERT_FILE is supported, untested against a real proxy)",
    "a symlinked game config file (it is replaced by a regular file; read from the code, not tested)",
    "Windows (paths, process control, config replacement)",
]))
c = d["codeMap"]
c["rules"] = [
    "files stay under 1000 lines (tests/repo_hygiene.rs); most under 300",
    "every mod.rs/lib.rs opens with a //! comment naming the module's job",
    "decisions are pure data (store/reduce, activity, notices, settings, nav::{route,transition}, window::{monitors,geometry,policy}, deck/state, surface/{presentation,menu,reveal}, shell/model, app_menu); components only draw",
    "hooks are never conditional: read tokens once at the top of a component",
    "one host vocabulary: Effect (the window's share is WindowCommand)",
    "form factor is decided only in surface::presentation",
    "shared state has one home (AppState); only reclaw-media fetches",
    "a change in behavior updates its spec in docs/specs in the same commit; a decision with a rejected alternative gets an ADR",
]
c["crates"] = {
    "reclaw-input": "Action, ActionMap, next_focus, HoldTracker, detect_environment, gilrs backend (feature)",
    "reclaw-runtime": "Supervisor, InputProfile, RunState",
    "reclaw-games": "Project, Platform, launch settings engine and config-file editors",
    "reclaw-config": "Preferences, tolerant load, atomic save, PrefsWriter",
    "reclaw-media": "address policy, sniffing, on-disk cache, MediaHub, README blocks",
    "reclaw-ui": "tokens, components, surface, store, nav, window, desktop, deck, shell",
}
c["doc"] = "ARCHITECTURE.md (module map, rules, testing map), AGENTS.md (commands and conventions), docs/README.md (specs and ADRs), docs/BUILDING.md (Bazzite and Linux builds)"
c["tools"] = ("design-system/tools: gen_tokens.py (tokens.json, freya/theme.rs, reclaw-ui/src/tokens.rs); contract: gen_contract_deck.py, gen_contract_surfaces.py, gen_contract_activity.py (this file, run last); "
              "previews and bundle: gen_components.py, gen_deck_components.py, gen_surfaces.py, gen_activity.py; render.py (screenshots)")
json.dump(d, open(P, "w"), indent=1)
print("contract updated:", len(d["components"]), "components,", len(d["decisions"]), "decisions")
