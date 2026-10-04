import os
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))  # work from design-system/, wherever this is run
import json
P="reclaw.freya.json"
d=json.load(open(P))
t=json.load(open("tokens.json"))
tok={x["name"]:x["value"]["midnight"] for x in t["color"]["tokens"]}
# refresh resolved theme tokens (now including deck-*)
d["theme"]["themes"]["midnight"]=tok
d["theme"]["themes"]["daylight"]={x["name"]:x["value"]["daylight"] for x in t["color"]["tokens"]}
d["theme"]["type"]["styles"]={s["name"]:{k:s[k] for k in ("fontSize","lineHeight","fontWeight")} for g in t["type"]["groups"] for s in g["styles"]}
d["theme"]["layout"]={x["name"]:x["value"] for x in t["layout"]["tokens"]}
d["status"]="buildable"
d["host"]="Library, Game page, Downloads, Catalog, Mods; Deck mode (controller-first shell)"

d["decisions"]=[
 {"id":"one-tree","decision":"One component tree. Input mode is an axis (Density::Controller beside Pointer and Touch) plus a small set of deck-only components. No parallel 'deck' copy of every component.","why":"Two trees drift. Shared components read density; only shelves, glyph hints, slide-in panels and the Play/Stop pair are new.","rejected":["separate component set per mode","a third color theme for deck"]},
 {"id":"deck-dark-only","decision":"Deck mode is dark only (deck-bg). Daylight is not offered.","why":"TV and handheld OLED viewing; halves the contrast matrix. Deck tokens carry the same value in both themes so lookups never fail."},
 {"id":"mode-vs-density","decision":"Layout class (width) and input density are independent. Deck mode forces the wide layout class and Density::Controller.","why":"A 1280x800 handheld and a 4K TV are both 'wide' but need 56-64px targets."},
 {"id":"input-ownership","decision":"The gamepad has one owner at a time: Launcher or App. While an app runs in front, the launcher ignores everything except Guide.","why":"Gamepad readers see every event whatever has window focus; without arbitration the launcher navigates its own menus while the user plays."},
 {"id":"no-steam-clone","decision":"Do not rebuild Steam's overlay or Steam Input. Reclaw owns Guide only when it is the shell; under Steam the Guide binding is removed.","why":"Steam's overlay and mapping already exist in Gaming Mode; two overlays on one button is worse than none."},
 {"id":"glyphs-follow-device","decision":"Hint glyphs follow the last-used device (gamepad kind, or keyboard keycaps). Face-button labels follow the cap (Nintendo A is East), actions follow position (Confirm is South unless swapped).","why":"Steam does this; showing 'A' to a PlayStation player is wrong."},
 {"id":"focus-is-declared","decision":"Focus targets are declared rectangles computed from layout constants, not measured from the rendered tree. A pure function picks the next target.","why":"Deterministic and unit-testable without a window; the same constants drive scroll offsets so the two cannot disagree."},
 {"id":"focus-not-hover","decision":"Hover never moves focus in Deck mode. Click = focus + confirm. The focus ring is visible only after gamepad or keyboard input.","why":"A resting mouse must not fight the gamepad; Steam hides the ring on mouse use."},
 {"id":"title-always","decision":"Tile titles are always visible; project, status badge and glow appear on focus, in space reserved up front.","why":"Placeholder art has no baked-in title, and reserved space avoids reflow on every focus move."},
 {"id":"overlays-trap","decision":"Main menu (left) and Quick Access (right) slide over the page, trap focus, and close with Back. They never shift the layout beneath.","why":"Matches the Big Picture model and keeps the page's focus position intact."},
 {"id":"stop-twice-forces","decision":"Stop sends a graceful quit; pressing it again while Stopping force-kills. Apps run in their own process group so the whole tree stops.","why":"Handhelds have no task manager; a hung game must be recoverable from the pad."},
 {"id":"resume-and-stop","decision":"Desktop swaps Play for Stop. Deck shows Resume + Stop side by side while an app runs.","why":"In a controller UI, Resume (A) is what you want 95% of the time, and Stop must not be one mis-press away."},
 {"id":"sdl-profile","decision":"Per-app controller setup is handed over as SDL_GAMECONTROLLERCONFIG plus SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS. Apps that do not read SDL need a virtual-device layer, which is not built.","why":"Most recompiled-game ports use SDL; this is the one mechanism that needs no kernel permissions."},
 {"id":"launcher-file-picker","decision":"Deck mode needs an in-app, gamepad-navigable file browser for 'your game file'. A native dialog cannot be driven by a pad.","why":"The install flow requires the user to choose their own file."},
 {"id":"text-input","decision":"Deck mode text entry uses an on-screen keyboard opened by Confirm on a field. Specified, not built.","why":"Every search and path field otherwise strands a gamepad user."}
]

d["modes"]={
 "uiMode":{"values":["Desktop","Deck"],"detect":{"order":["RECLAW_MODE=deck|desktop (always wins)","SteamDeck is set and not 0","SteamGamepadUI is set and not 0","XDG_CURRENT_DESKTOP contains 'gamescope'","GAMESCOPE_WAYLAND_DISPLAY is set","else Desktop"],"unverified":"The Steam and gamescope variables are the commonly documented ones; not confirmed on real SteamOS hardware. RECLAW_MODE exists to override a wrong guess.","rust":"reclaw_input::detect_environment"}},
 "guideOwner":{"values":["Reclaw","Steam"],"rule":"Steam when SteamGameId or SteamAppId is set (launched as a Steam shortcut). Then the Guide button binding is removed (ActionMap::unbind) and Steam's overlay handles it.","rust":"reclaw_input::GuideOwner"},
 "density":{
  "Pointer":{"rowHeight":32,"button":32,"body":14,"minTarget":32,"hover":True,"focusRing":"keyboard only"},
  "Touch":{"rowHeight":48,"button":48,"body":16,"minTarget":44,"hover":False,"focusRing":"none"},
  "Controller":{"rowHeight":64,"button":56,"body":20,"minTarget":56,"hover":False,"focusRing":"always after pad/keyboard input","typeScale":"deck-*"}},
 "deployment":[
  {"id":"steam-gaming-mode","how":"Added to Steam as a non-Steam shortcut","guide":"Steam","overlay":"Steam","glyphs":"Xbox (Steam Input presents a virtual Xbox 360 pad)","reclawBuilds":"navigation, tiles, lifecycle, hint bar"},
  {"id":"standalone-console","how":"Reclaw is the shell (own gamescope session or a handheld Linux distro)","guide":"Reclaw","overlay":"Reclaw MainMenu and QuickAccess","glyphs":"detected from the device","reclawBuilds":"everything above plus Guide handling"},
  {"id":"desktop","how":"Normal window","guide":"unused","overlay":"none","glyphs":"keyboard keycaps until a pad is used"},
  {"id":"windows-handheld","how":"Standalone on a Windows handheld","guide":"mostly unreachable through XInput","glyphs":"detected","note":"Guide-button capture is not solved on Windows; use Select as the Quick Access key and Start+Select for MainMenu if confirmed necessary. Untested."}]
}

d["input"]={
 "owner":"reclaw_input::InputMapper",
 "actions":[
  {"action":"Navigate(Up|Down|Left|Right)","from":"D-pad, left stick","repeat":True},
  {"action":"Confirm","from":"South (East when swapped)","glyph":"the cap on the Confirm button: A, cross, or B on a Nintendo pad (A when the layout is swapped)"},
  {"action":"Back","from":"East (South when swapped)"},
  {"action":"Secondary","from":"West","meaning":"focused item's second verb: Manage"},
  {"action":"Tertiary","from":"North","meaning":"Search"},
  {"action":"PrevSection","from":"Left bumper"},{"action":"NextSection","from":"Right bumper"},
  {"action":"PageUp","from":"Left trigger"},{"action":"PageDown","from":"Right trigger"},
  {"action":"MainMenu","from":"Guide","passesWhileAppOwnsPad":True},
  {"action":"QuickAccess","from":"Select (View / Create / -)"},
  {"action":"Options","from":"Start (Menu / Options / +)","meaning":"context menu of the focused item; not built"}],
 "rebind":"ActionMap::rebind, swapped_confirm_back (Nintendo layout), unbind",
 "keyboardFallback":{"Arrows":"Navigate","Enter":"Confirm","Escape":"Back","PageUp/PageDown":"PageUp/PageDown","Tab":"MainMenu","Shift+Tab":"QuickAccess","[ and ]":"PrevSection / NextSection"},
 "stick":{"enter":0.6,"exit":0.4,"snap":"4-way by dominant axis","hysteresis":"holds the direction between exit and enter","yAxis":"+Y up (gilrs normalizes this; verified in its source)"},
 "repeat":{"delayMs":400,"intervalMs":110,"stall":"one move per tick even after a long hitch","dpadPriority":"newest held D-pad direction wins, falls back to the older one on release; D-pad beats stick"},
 "ownership":{"values":["Launcher","App"],"App":"swallows every button and axis except Guide (MainMenu); clears held directions on every switch so nothing phantom-repeats","switchedBy":"DeckState::input_owner(): App while an app is active and the launcher is not in front"},
 "lastInput":{"values":["Gamepad(kind)","Keyboard","Pointer"],"drives":["hint glyph set","focus ring visibility"]},
 "controllerKinds":{"detect":"USB vendor id first (045e Xbox, 054c PlayStation, 057e Nintendo, 28de Valve), then name","kinds":["Xbox","PlayStation","Nintendo","SteamDeck","Generic"],"rust":"reclaw_input::ControllerKind::detect"},
 "glyphs":{"Xbox":{"South":"A","East":"B","West":"X","North":"Y","bumpers":"LB RB","triggers":"LT RT","Select":"View","Start":"Menu","Guide":"Guide"},
  "PlayStation":{"South":"cross icon","East":"circle icon","West":"square icon","North":"triangle icon","bumpers":"L1 R1","triggers":"L2 R2","Select":"Create","Start":"Options","Guide":"PS","render":"Lucide x, circle, square, triangle: no font has to carry the symbols"},
  "Nintendo":{"South":"B","East":"A","West":"Y","North":"X","bumpers":"L R","triggers":"ZL ZR","Select":"-","Start":"+","Guide":"Home"},
  "SteamDeck":{"face":"A B X Y","bumpers":"L1 R1","triggers":"L2 R2","Select":"View","Start":"Menu","Guide":"Steam"},
  "Generic":"Xbox letters with LB RB LT RT","glyphStyle":"neutral: bg-raised circle, line-strong ring, ink letter (no vendor colors; avoids contrast and trademark questions)"},
 "appProfile":{"rust":"reclaw_runtime::InputProfile + SdlMapping","sets":["SDL_GAMECONTROLLERCONFIG (full mapping line)","SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS=1","extra env"],"remap":"SdlMapping::parse then swap(a, b) rewrites a device's mapping, e.g. A/B for Nintendo layout","covers":"SDL apps; N64Recomp-style ports are expected to be SDL-based, to be checked per project","notCovered":"apps that bypass SDL: needs a virtual keyboard/mouse device (uinput), which needs /dev/uinput permission and is not built"},
 "backend":{"rust":"reclaw_input::backend (feature gilrs-backend)","thread":"dedicated; Gilrs is not Send","tick":"wakes every 8ms for repeats","status":"compile-checked; not run against hardware (no /dev/input in the dev container)"}
}

d["focus"]={
 "model":"declared rectangles + pure spatial function",
 "rust":"reclaw_input::next_focus, reclaw_ui::deck::DeckState",
 "rule":{"candidate":"center must lie beyond the current center in the pressed direction (> 0.5px)","score":"along + 2 * crossCenterOffset + 4 * crossGap","tie":"lower FocusId","edge":"no wrap; nothing happens at an edge","why":"Down from a tile lands on the tile underneath, not the nearest tile in the next row"},
 "scopes":["Home(section)","Game(id)","MainMenu","QuickAccess"],
 "scopeRules":{"overlayTrap":"while an overlay is open only its nodes are candidates","memory":"leaving a scope remembers its focus; Back restores it","defaultFocus":{"Home":"first tile of the first shelf, or the NowPlayingBanner when an app runs","Game":"LaunchButton primary","MainMenu":"current section's row","QuickAccess":"Resume when an app runs, else the first row"}},
 "backStack":"Overlay > Game page > Home. Back on Home does nothing.",
 "pointer":"click = focus + confirm; hover does not move focus",
 "focusVisual":{"tile":"scale deck-focus-scale (1.06), 3px accent border (inner), outer glow focus-glow blur 24 spread 2","button":"scale 1.04, 3px accent border, glow","row":"bg-raised fill + 4px accent bar on the left, no scale","ms":120,"ease":"out","visibleWhen":"lastInput is Gamepad or Keyboard"}
}

d["lifecycle"]={
 "rust":"reclaw_runtime::Supervisor / RunState / Outcome",
 "states":[
  {"state":"Idle","enter":"app not running, or ended cleanly"},
  {"state":"Starting","enter":"UI sets optimistically on press; cleared by the Started event (spawn is synchronous so it is brief)"},
  {"state":"Running","data":"pid, since","enter":"SessionEvent::Started"},
  {"state":"Stopping","data":"pid","enter":"Stop pressed; graceful quit sent to the whole process group"},
  {"state":"Failed","data":"Outcome","enter":"ended with a non-zero code or a foreign signal; cleared on next launch"}],
 "outcomes":[
  {"outcome":"Exited","ui":"back to Idle, no message"},
  {"outcome":"Stopped","ui":"back to Idle, no message"},
  {"outcome":"ExitedWithCode(n)","ui":"Failed: 'Exited with code n', button Retry, log link. Many games exit non-zero on a normal quit, so this is a notice, not 'crashed'."},
  {"outcome":"Signaled(sig)","ui":"Failed: 'Closed unexpectedly (signal sig)', button Retry, log link"}],
 "stop":{"first":"SIGTERM to the process group (Windows: taskkill /T)","grace":"5s default; then SIGKILL to the group","second":"pressing Stop while Stopping force-kills immediately","pidReuseGuard":"the exit flag is set before events are sent, so the watchdog never signals a reaped pid (a microsecond window remains)"},
 "oneSession":"start() refuses a second launch of the same app (AlreadyRunning); the table is held across spawn so two quick presses cannot launch two copies",
 "logs":"LaunchSpec.log appends stdout and stderr to a file; shown from the Failed state",
 "inputOwnership":{"onLaunch":"launcher goes to the background; owner = App","onGuide":"launcher comes to front, MainMenu opens, owner = Launcher","onResume":"launcher goes to the back, owner = App","onAppEnded":"launcher to front, owner = Launcher; a Failed outcome shows a toast on the game's tile and page","windowing":"bringing windows forward is platform glue (not built); the model emits Effects and tracks inFront"},
 "ui":{
  "LaunchButton":[
   {"when":"Available or NeedsFile","desktop":"Install (install variant)","deck":"Install"},
   {"when":"Installing","desktop":"Installing (disabled)","deck":"Installing (disabled)"},
   {"when":"UpdateReady and Idle","desktop":"Update","deck":"Update"},
   {"when":"Installed and Idle","desktop":"Play","deck":"Play"},
   {"when":"Starting","desktop":"Starting (disabled)","deck":"Starting (disabled)"},
   {"when":"Running","desktop":"Stop (danger variant, square icon) replaces Play","deck":"Resume (install variant) + Stop (danger) side by side"},
   {"when":"Stopping","desktop":"Force quit (danger, enabled)","deck":"Force quit (danger, enabled)"},
   {"when":"Failed","desktop":"Retry (install variant) + message line","deck":"Retry + message line"}],
  "badge":"Running (ok tone, play icon) takes priority over Installed on tiles and the hero",
  "NowPlayingBanner":"shown at the top of Deck Home while an app is active"},
 "platforms":{"unix":"process groups via killpg; tested","windows":"taskkill /T [/F]; written, not tested","escapes":"an app that calls setsid or double-forks leaves the group and is not stopped"}
}

# ---- components
def comp(name,freya,tokens,**kw):
    c={"name":name,"freya":freya,"tokens":tokens}; c.update(kw); return c
deck=[
 comp("DeckTile","custom",["bg-panel","deck-scrim","accent","focus-glow","ink","ink-muted","ok","ok-bg"],
  layout="col",size={"art":"deck-tile-w x deck-tile-h (3:4)","caption":"reserved 72px: title (deck-label) + StatusBadge"},radius="4,4,4,4",
  props={"game":"GameEntry","focused":"bool","selected":"bool"},states=["rest","focused","running"],
  rules=["title is always visible, one line, ellipsis","the StatusBadge is drawn only when focused or running (space reserved, so focus never reflows); the project name is on the game page","focused: scale deck-focus-scale, 3px accent inner border, glow shadow blur 24 spread 2 in focus-glow","text on art sits on deck-scrim; only ink and ink-muted are allowed there","Running badge always shows"],
  freyaApi="rect().scale((s,s)).border(Border::new().fill(accent).width(3.).alignment(Inner)).shadow(Shadow::new().blur(24.).spread(2.).color(focus_glow))",
  anim={"name":"focus","ms":120,"ease":"out"},rust={"type":"DeckTile","module":"reclaw_ui::deck"}),
 comp("Shelf","custom",["ink","ink-muted"],layout="col",
  props={"title":"String","games":"Vec<GameEntry>","focused":"Option<FocusId>","viewport_w":"f32"},
  rules=["title in deck-heading","tiles in a clipped row; offset_x keeps the focused tile centered, clamped to [0, content - viewport]","the same offset math builds the declared focus rectangles"],
  freyaApi="rect().horizontal().offset_x(-offset) inside an Overflow::Clip parent",rust={"type":"Shelf","module":"reclaw_ui::deck"}),
 comp("ButtonGlyph","custom",["bg-raised","line-strong","ink"],size={"diameter":28},
  props={"face":"GlyphFace","keyboard":"Option<&str> (keycap form)"},
  rules=["neutral: bg-raised circle, line-strong 2px ring, ink letter in deck-hint","PlayStation faces are Lucide x / circle / square / triangle in ink","keycap form: rounded rect, label in mono"],rust={"type":"ButtonGlyph","module":"reclaw_ui::deck"}),
 comp("HintBar","custom",["deck-bg","ink","ink-muted","line"],layout="row",size={"height":"deck-hint-h"},
  props={"hints":"Vec<(Action, label)>","kind":"ControllerKind","last_input":"LastInput"},
  rules=["sits in the bottom safe zone; hints are contextual to the screen and overlay","glyphs from ActionMap::glyph(action, kind), so a rebind updates the bar","keyboard keycaps when last input was the keyboard"],
  freyaApi="rect().horizontal() + ButtonGlyph + label",rust={"type":"HintBar","module":"reclaw_ui::deck"}),
 comp("SlidePanel","custom",["bg-panel","line","scrim"],layout="overlay",size={"width":"deck-panel-w"},
  props={"side":"Left|Right","open":"bool","child":"Element"},
  rules=["absolute overlay: never shifts the page beneath","scrim token fills the rest of the window while open","open and close slide 280ms expo-out; scrim fades 200ms","traps focus while open; Back closes"],
  freyaApi="rect().position(Position::new_absolute().top(0).left(x)) driven by use_animation_transition(open)",anim={"name":"panel","ms":280,"ease":"expo-out"},rust={"type":"SlidePanel","module":"reclaw_ui::deck"}),
 comp("MainMenu","custom",["bg-panel","bg-raised","accent","ink"],layout="col",
  items=["Library","Catalog","Downloads","Mods","Settings (not built)","Switch to desktop mode"],
  rules=["rows deck-row-h, icon + deck-body label","focused row: bg-raised + 4px accent bar, no scale","opens on MainMenu (Guide)"],rust={"type":"MainMenu","module":"reclaw_ui::deck"}),
 comp("QuickAccess","custom",["bg-panel","ink","ink-muted","ok"],layout="col",
  sections=["Now playing: title, elapsed, Resume + Stop (only while an app is active)","Controller: name, kind, battery (read-only)","Downloads: 'n in queue' row, opens Downloads"],
  rules=["opens on QuickAccess (Select)","default focus is Resume when an app is active"],rust={"type":"QuickAccess","module":"reclaw_ui::deck"}),
 comp("LaunchButton","ActionButton",["install","install-hover","on-install","danger","danger-bg"],
  props={"status":"AppStatus","run":"RunState","density":"Density","on_primary":"handler","on_stop":"handler"},
  rules=["see lifecycle.ui.LaunchButton for the state table","Controller density renders Resume + Stop as a pair","Stopping shows Force quit (enabled); a second press force-kills"],
  rust={"type":"LaunchButton","module":"reclaw_ui::deck"}),
 comp("NowPlayingBanner","custom",["bg-panel","ok","ink","accent"],layout="row",content="Content::Flex",
  props={"game":"GameEntry","run":"RunState"},
  rules=["top of Deck Home while an app is active","title + 'Running, 12 min' + Resume + Stop; both buttons are focus targets"],rust={"type":"NowPlayingBanner","module":"reclaw_ui::deck"}),
 comp("SectionTabs","custom",["ink","ink-muted","accent"],layout="row",size={"height":"deck-tabs-h"},
  items=["Library","Catalog","Downloads","Mods"],
  rules=["bumper glyphs flank the strip (PrevSection / NextSection)","selected tab: ink + 4px accent underline; deck-heading","not a focus target; bumpers move it"],rust={"type":"SectionTabs","module":"reclaw_ui::deck"}),
 comp("Backdrop","custom",["deck-bg","deck-scrim","accent"],layout="stack",
  props={"tint":"Color (dominant color of the focused game's art; accent when unknown)"},
  rules=["full-bleed behind the page; follows the focused game","art upscaled and blurred (EffectExt::blur 24), covered by deck-scrim so text is always on the scrim","without art: deck-bg with a vertical tint wash","never carries text itself"],rust={"type":"Backdrop","module":"reclaw_ui::deck"}),
 comp("DeckApp","custom",["deck-bg"],
  rules=["root: measures width with on_sized (set_if_modified), owns DeckState, drains the ActionFeed and the keyboard fallback, runs the reducer and hands Effects to the host callback","safe zone padding deck-safe-x / deck-safe-y on every page","HintBar pinned in the bottom safe zone"],rust={"type":"DeckApp","module":"reclaw_ui::deck"}),
 comp("FileBrowser","custom",[],status="specified, not built",rules=["gamepad-navigable replacement for the native file picker in InstallDialog","list rows deck-row-h; A opens folder or selects file; B goes up"]),
 comp("OnScreenKeyboard","custom",[],status="specified, not built",rules=["opened by Confirm on a text field in Controller density","grid of 56px keys, spatial navigation, A types, X backspace, Y shift, Start done"]),
]
names={c["name"] for c in deck}
d["components"]=[c for c in d["components"] if c["name"] not in names]+deck

# per-component Controller density adaptations for the shared set
adapt={
 "Button":"Controller: height 56 (hero 64), deck-label 18/24, icon 22, focus: ring + scale 1.04",
 "SearchField":"Controller: renders as a button-like field; Confirm opens the OnScreenKeyboard (not built)",
 "InstallDialog":"Controller: focus trapped, buttons 56px, Back closes, Confirm runs the focused button; 'Browse' opens FileBrowser (not built) instead of the native picker",
 "HeroHeader":"Controller: not used; DeckGame composes Backdrop + deck-title + LaunchButton pair",
 "LibraryRow":"Controller: not used; Deck uses DeckTile and MainMenu rows",
 "Chip":"Controller: not used; SectionTabs replaces it",
 "Nav":"Controller: not used; SectionTabs + MainMenu replace it",
 "StatusBadge":"Controller: same component, 24px high, deck-hint size text; adds Running",
 "DownloadItem":"Controller: row deck-row-h x 1.5, deck-body, Cancel is a focus target",
 "GameCapsule":"Controller: not used; DeckTile",
 "Switch":"Controller: 56px row, Confirm toggles",
}
for c in d["components"]:
    if c["name"] in adapt: c["controller"]=adapt[c["name"]]

d["layout"]["classes"]=[x for x in d["layout"]["classes"] if x.get("id")!="deck"]+[
 {"id":"deck","how":"UiMode::Deck; forces wide width class and Density::Controller","safeZone":{"x":"deck-safe-x","y":"deck-safe-y"},
  "screens":[
   {"id":"DeckHome","regions":["SectionTabs (top)","NowPlayingBanner (when an app is active)","Shelf 'Continue' (installed or running)","Shelf 'All apps'","HintBar (bottom)"],"hints":["Confirm Select","Back (none on Home)","PrevSection/NextSection Section","QuickAccess Quick access","MainMenu Menu"]},
   {"id":"DeckGame","regions":["Backdrop","deck-title + project eyebrow","LaunchButton (primary, + Stop when running)","secondary row: Open folder, Manage","meta: version, source, status","HintBar"],"hints":["Confirm Select","Back Library","Secondary Manage"]},
   {"id":"Downloads","regions":["DownloadItem list, Cancel focusable"]},
   {"id":"Catalog / Mods","status":"empty state only (not built)"}],
  "overlays":["MainMenu (left, deck-panel-w)","QuickAccess (right, deck-panel-w)","InstallDialog (centered, focus trapped)"]}]
d["responsive"]["deck"]="Deck is a mode, not a breakpoint: it ignores width classes and scales by safe-zone margins and a user UI scale (1.0 / 1.25 / 1.5 for TV). Pointer and touch still work; they just do not drive focus."

d["dataModel"].update({
 "RunState":"Idle | Starting | Running{pid,since} | Stopping{pid} | Failed(Outcome)",
 "Outcome":"Exited | ExitedWithCode{code} | Signaled{signal} | Stopped (each with ran_for)",
 "ControllerInfo":"{id,name,kind: Xbox|PlayStation|Nintendo|SteamDeck|Generic,vendor_id,product_id,power: Unknown|Wired|Charging(p)|Discharging(p)|Full}",
 "DeckState":"{section, screen: Home|Game(id), overlay: None|MainMenu|QuickAccess, focus, memory per scope, in_front, last_input}",
 "Effect":"Launch(id) | Resume(id) | Stop(id) | Install(id) | OpenFolder(id) | Manage(id) | Search | SwitchToDesktop | BringLauncherToFront | SendLauncherToBack",
 "GameEntry.run":"RunState (added)"})

d["animations"]=[a for a in d["animations"] if a["name"] not in ("focus","panel","scrim")]+[
 {"name":"focus","kind":"scale+glow","ms":120,"ease":"out","drives":"DeckTile and button focus"},
 {"name":"panel","kind":"translate-x","ms":280,"ease":"expo-out","drives":"MainMenu (left) and QuickAccess (right)"},
 {"name":"scrim","kind":"opacity","ms":200,"ease":"out","drives":"SlidePanel backdrop"}]

d["notBuilt"]=[
 "OnScreenKeyboard","FileBrowser (gamepad file picker)","Options context menu",
 "Catalog and Mods sections (empty state only)","Settings screen",
 "Virtual keyboard/mouse device (uinput) for apps that do not read SDL",
 "Bringing the launcher/app window forward (platform glue; the model emits Effects)",
 "Windows Guide-button capture and Windows job-object process control (taskkill fallback is untested)",
 "Hardware test of the gilrs backend",
 "Big Art from real catalog images and dominant-color extraction (Backdrop takes a tint and an optional image)"]
for c in d["components"]:
    if c["name"] in ("DeckTile","Shelf","ButtonGlyph","HintBar","SlidePanel","MainMenu","QuickAccess","LaunchButton","NowPlayingBanner","SectionTabs","Backdrop","DeckApp"):
        c["status"]="built"
d["verification"]={
 "freya":"0.5.0-rc.8",
 "workspace":"reclaw-input (26 tests), reclaw-runtime (14), reclaw-ui (25 unit, 5 deck snapshots incl. a keyboard-driven reducer test, 3 desktop snapshots, 3 token drift tests, 1 end-to-end lifecycle test)",
 "endToEnd":"tests/deck_lifecycle.rs drives the real DeckApp with key presses against a real process: Play launches it and hands the pad to the app (InputOwner(App)); Guide brings the launcher forward (BringLauncherToFront, InputOwner(Launcher)); Back sends it back; Stop ends the process group; the app ending brings the launcher forward again.",
 "driftGuard":"tests/tokens_in_sync.rs fails if any color token (both themes), spacing, radius, layout or deck type size in tokens.json differs from the Rust constants.",
 "lint":"cargo clippy --workspace --all-targets --features reclaw-ui/gamepad -- -D warnings is clean",
 "notVerified":["gilrs backend against real hardware (no /dev/input in the dev container)","Steam and gamescope environment variables on SteamOS","Windows process control","SDL_GAMECONTROLLERCONFIG against a real SDL game","visual check beyond headless Skia snapshots on a TV or handheld"]}
json.dump(d,open(P,"w"),indent=1)
print(len(d["components"]),"components;",len(d["decisions"]),"decisions")
