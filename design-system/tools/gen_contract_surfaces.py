"""Extends reclaw.freya.json with the surface system, interface switching, the settings page and the code map.
Idempotent: every entry is keyed and replaced, never appended twice. Run from anywhere:
`python3 design-system/tools/gen_contract_surfaces.py`, after gen_tokens.py."""
import os, json
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
P = "reclaw.freya.json"
d = json.load(open(P))
t = json.load(open("tokens.json"))

def upsert(items, entry, key):
    for i, x in enumerate(items):
        if x[key] == entry[key]:
            items[i] = entry
            return
    items.append(entry)

# ---- tokens: every layout token, including the surface-* ones
d["theme"]["layout"] = {x["name"]: x["value"] for x in t["layout"]["tokens"]}

# ---- data model
dm = d["dataModel"]
dm["DeckState"] = ("{section, screen: Home|Game(id)|Install(id)|Settings(Global|App(id)), stack, overlay: None|MainMenu|QuickAccess|Menu(Options(id)|Choice(target,key))|Confirm(Uninstall(id)), "
                   "focus, memory per scope, in_front, last_input, owner, window:(w,h), menu: Option<MenuState<MenuAction>>, settings_section, drilled, values: SettingsValues, install: InstallDraft, text entry: Option<TextField>}")
dm["Effect"] = ("Launch(id) | Resume(id) | Stop(id) | StartInstall{app,location,game_file,shortcut,prerelease} | Update(id) | ChooseFile(id) | OpenFolder(id) | Verify(id) | CheckUpdate(id) | Uninstall(id) | "
                "ToggleFavorite(id) | AddToLibrary(id) | RemoveFromLibrary(id) | RefreshCatalog | CancelDownload(id) | Search | SwitchToDesktop | SetMode(Auto|Desktop|Deck) | Setting(change) | BeginTextEntry(field) | "
                "EndTextEntry(field) | TextCommitted{app,field,value} | BringLauncherToFront | SendLauncherToBack | InputOwner(Launcher|App). Defined in reclaw_ui::effect, shared by both interfaces. "
                "SubmitInstall is internal: DeckApp turns it into StartInstall because it holds the form's text.")
dm["HostState"] = "{games: State<Vec<GameEntry>>, downloads: State<Vec<Download>>, controller: State<Option<ControllerInfo>>, keyboard_inset: State<f32>, chosen_file: State<Option<String>>}. The host creates and writes it; both interfaces read it."
dm["MenuAction"] = "ToggleFavorite | AddToLibrary | RemoveFromLibrary | OpenFolder | Verify | CheckUpdate | Uninstall | Properties | Cancel | Choice(i). reclaw_ui::app_menu; options_menu(game, with_properties) builds the menu."
dm["Settings"] = "Schema{title, sections:[Section{id,title,groups:[Group{heading?,note?,rows:[Row{key,label,description?,kind: Toggle|Choice|Text|Action|Info}]}]}]}; values keyed (target: Global|App(id), key). reclaw_ui::deck::settings."

# ---- surfaces
presentation = [
    {"kind": "Form", "means": "inputs, install, settings, properties", "desktop": "Popup over a dimmed page", "touchTall": "Popup (touch density and at least surface-touch-popup-min-h tall)",
     "compact": "FullScreen page: phone width, any controller, or touch density under surface-touch-popup-min-h tall"},
    {"kind": "Confirm", "means": "a sentence and two buttons, nothing to scroll", "desktop": "Popup", "touchTall": "Popup", "compact": "Centered card over a darkened screen (phone width or controller density)"},
    {"kind": "Menu", "means": "a list of choices that may cascade", "desktop": "Anchored popover at the press point (pointer density, not phone width)", "touchTall": "Centered over a darkened screen",
     "compact": "Centered over a darkened screen"},
]
d["surfaces"] = {
    "rust": "reclaw_ui::surface; presentation() is the one table. Screens never branch on form factor themselves.",
    "kinds": ["Form", "Confirm", "Menu"],
    "presentation": presentation,
    "context": {"class": "LayoutClass Wide|Compact|Phone", "density": "Pointer|Touch|Controller", "height": "window height in px",
                "isCompact": "class == Phone || density == Controller || (density == Touch && height < surface-touch-popup-min-h)"},
    "fullScreenPage": {
        "rust": "surface::FullScreenPage (Dialog picks it)",
        "anatomy": ["header: surface-header-h, Back button (ghost, chevron-left, label Back) + title, one line with ellipsis",
                    "body: ScrollView in a centered column capped at surface-max-w; `wide` removes the cap; `fixed` hands the body its own scrolling (two-pane)",
                    "footer: surface-footer-h, actions right-aligned, top hairline",
                    "hint bar (Deck only): DeckHintBar with surface-hints-pad-b below and deck-safe-x either side"],
        "shortWindow": "window height under surface-short-h: the footer's actions move into the header, right-aligned, and the footer row is dropped",
        "back": "header Back button, the B button and Esc are the same action; Back is focus target PAGE_BACK in Deck mode",
        "keyboardAvoidance": {
            "input": "HostState.keyboard_inset (px; 0 = hidden). The OS binding writes it; F9 and RECLAW_KEYBOARD simulate it.",
            "whileUp": ["footer and hint bar are not drawn", "body ends in a spacer as tall as the keyboard so the last field can scroll clear",
                        "header height becomes surface-header-h-compact when inset > 40% of the window height",
                        "the focused field's [top,bottom] in body coordinates is scrolled into [0, visible_height] with a 16px margin, moving as little as possible; a node taller than the viewport aligns to its top"],
            "math": "visible_height = window - header - (footer + hints, only when the keyboard is down) - inset; scroll_to_reveal(scroll, top, bottom, viewport, margin). Both are pure and unit-tested (surface::reveal).",
            "scrollCoordinates": "Freya's scroll controller stores the negative of the offset; RevealScroll converts",
        },
    },
    "popup": "Freya Popup, 480 wide (Confirm 420); title, content, right-aligned buttons. Closing animation needs the Popup to stay mounted while closed; the other presentations simply unmount.",
    "centeredCard": "Confirm on touch/controller: scrim token over the window, card width min(window-48, 420), radius-lg, space-5 padding, actions right-aligned, title in deck-heading",
    "modalMenu": {
        "rust": "surface::ModalMenu with MenuState<A> (surface::menu, pure)",
        "centered": {"scrim": "rgba(5,8,12,0.85)", "title": "deck-heading above the menu", "columnWidth": "deck-menu-w (min(window-48, deck-menu-w))", "rowHeight": "deck-row-h",
                     "focusedRow": "inverted: ink fill, deck-bg text", "openParentRow": "ink-muted fill, deck-bg text", "separator": "4px gap of deck-bg above a row", "disabled": "ink-subtle text, skipped by Up/Down",
                     "cascade": "a `>` row opens its items in a new column to the right; parents stay visible. When the window is narrower than 2*deck-menu-w + 96 only the deepest column shows.",
                     "maxHeight": "column scrolls past window height - 200"},
        "anchored": {"width": "280", "rowHeight": "40", "border": "1px line, radius-md, shadow-pop", "position": "press point, slid left/up to stay inside the window", "dismiss": "press outside"},
        "behavior": {"navigate": "Up/Down stop at the ends and skip disabled rows", "right/confirm": "on `>` opens the submenu with its first enabled row focused", "left/back": "closes one level; at the root dismisses",
                     "pointer": "pick(level, index) is the same as moving there and confirming", "outcomes": "None | Moved | Chose(action) | Closed"},
        "options": "Add to favorites | Add to library (only for a project not in the library) | Manage > (Open install folder, Verify files, Check for updates: disabled until installed; Remove from library: library entries only; Uninstall: disabled until installed) | Properties... | Cancel. Properties is left out where no Properties page exists (desktop).",
    },
    "settingsPage": {
        "rust": "deck::SettingsBody inside FullScreenPage; schema in deck::settings",
        "twoPane": "window width >= deck-two-pane-min-w: section list (deck-settings-nav-w) beside the rows; rows scroll independently",
        "drillDown": "narrower: the section list alone; pressing a section shows its rows; Back goes list <- rows <- leave",
        "row": {"height": "deck-settings-row-h (+72 for a text row)", "text": "one line of label, one of description (ellipsis; keep descriptions short)", "controls": "Toggle (switch), Choice (value + chevron, opens a centered menu), Info, Text (input under the label), Action (the row)",
                "focus": "bg-raised fill and deck-focus-ring accent border"},
        "geometry": "section_slots(section, density) gives every row's y and height; the focus layout and the renderer both read it, and group headings (48) and notes (32) and gaps (24) are in it",
    },
    "textEntry": {
        "enter": "Confirm on a Text row (pad or keyboard) starts typing: the box takes keyboard focus and is cleared, its old text kept aside. A pointer or touch press keeps the text and places the caret.",
        "why": "Freya 0.5 Input starts its caret at the beginning and cannot be moved from outside, so typing would land in front of the old text.",
        "leave": "Confirm or Back ends entry. TextCommitted carries the value; an empty box gets its old text back.",
        "whileTyping": "only Confirm and Back are the interface's; the hint bar shows just Done; the root is not a focus target so Tab cannot take the keyboard away",
        "alternativesRejected": "a custom caret-aware text box (IME, clipboard, selection to rebuild); selecting all on focus (not exposed)",
    },
    "focusManagement": "Deck mode moves focus itself. Buttons under DeckApp get ManagedFocus and are not in Freya's Tab order: a button holding Freya focus would also fire on Enter, doubling the Confirm.",
}

d["shell"] = {
    "rust": "reclaw_ui::shell::Shell (root), ShellModel (pure), DevOverrides",
    "interfaces": ["Desktop (DesktopApp: Library at the layout class and density the window calls for)", "Deck (DeckApp)"],
    "start": "reclaw_input::detect_environment: RECLAW_MODE, SteamDeck, SteamGamepadUI, gamescope variables (unverified on real SteamOS); Desktop otherwise",
    "enterByHand": [
        {"how": "F10", "does": "toggles the interface; the choice sticks as an explicit preference"},
        {"how": "Deck mode button in the desktop top bar (icon + label; below Wide it sits beside the search field)", "does": "switches to Deck"},
        {"how": "Deck main menu > Switch to desktop mode", "does": "Effect::SwitchToDesktop"},
        {"how": "Settings > Interface: Auto | Desktop | Deck", "does": "Effect::SetMode; Auto returns to what was detected"},
    ],
    "simulateKeyboard": [{"how": "F9", "does": "show or hide a simulated on-screen keyboard (45% of the window height)"},
                         {"how": "RECLAW_KEYBOARD=auto|<px>", "does": "start with it up"}, {"how": "RECLAW_SIM_KEYBOARD=1", "does": "raise it whenever a text box takes focus, like a touch device"}],
    "overrides": {"RECLAW_MODE": "deck|desktop", "RECLAW_LAYOUT": "wide|compact|phone (desktop)", "RECLAW_DENSITY": "pointer|touch|controller", "RECLAW_THEME": "midnight|daylight"},
    "stateAcrossSwitch": "HostState is shared, so games, downloads, run states and the keyboard survive; each interface's own navigation state starts fresh. A mode switch drops the simulated keyboard.",
    "pad": "the action feed gives its receiver back when Deck mode unmounts and discards presses made meanwhile, so the pad works again on re-entry and nothing replays",
}

d["codeMap"] = {
    "doc": "ARCHITECTURE.md (module map, rules, testing map) and AGENTS.md (commands and conventions)",
    "rules": ["files stay under 1000 lines (tests/repo_hygiene.rs); most under 300", "every mod.rs/lib.rs opens with a //! comment naming the module's job",
              "decisions are pure data (deck/state, deck/settings, surface/{presentation,menu,reveal}, shell/model, app_menu); components only draw",
              "hooks are never conditional: read tokens once at the top of a component", "one host vocabulary: Effect", "form factor is decided only in surface::presentation"],
    "crates": {"reclaw-input": "Action, ActionMap, next_focus, detect_environment, gilrs backend (feature)", "reclaw-runtime": "Supervisor, InputProfile, RunState", "reclaw-ui": "tokens, components, surface, deck, shell"},
    "tools": "design-system/tools: gen_tokens.py (tokens.json, freya/theme.rs, reclaw-ui/src/tokens.rs), gen_contract_deck.py + gen_contract_surfaces.py (this file), gen_components.py / gen_deck_components.py / gen_surfaces.py (previews and bundle), render.py (screenshots)",
}

# ---- components
def comp(name, freya, rust, **kw):
    e = {"name": name, "freya": freya, "rust": {"type": rust.split("::")[-1], "module": "reclaw_ui::" + "::".join(rust.split("::")[:-1]), "built": "custom"}}
    e.update(kw)
    return e

upsert(d["components"], comp("InstallDialog", "Popup | FullScreenPage via Dialog", "components::InstallDialog",
    tokens=["bg-panel", "line-strong", "install", "ink", "ink-muted"], action="install",
    note="Adaptive: a Popup on desktop, a FullScreenPage on phones and short touch screens (and the Deck Install page on a controller). Install is disabled until the user has chosen their own game file.",
    props={"open": "bool", "surface": "SurfaceContext", "window": "(f32,f32)", "keyboard_inset": "f32", "location": "State<String>", "game_file": "State<Option<String>>", "shortcut": "State<bool>", "prerelease": "State<bool>"}), "name")
upsert(d["components"], comp("Dialog", "Popup | FullScreenPage | centered card", "surface::Dialog", kind="surface",
    props={"kind": "SurfaceKind", "ctx": "SurfaceContext", "window": "(w,h)", "title": "String", "body": "Element", "actions": "Vec<DialogAction>", "on_close": "EventHandler<()>", "keyboard_inset": "f32", "reveal": "Option<(top,bottom)>"},
    note="Describe the content and actions once; presentation(kind, ctx) picks the form."), "name")
upsert(d["components"], comp("FullScreenPage", "rect + ScrollView", "surface::FullScreenPage", kind="surface",
    tokens=["bg-nav", "deck-bg", "line", "ink", "ink-muted"], layout="col",
    size={"header": "surface-header-h (surface-header-h-compact under a large keyboard)", "footer": "surface-footer-h", "column": "surface-max-w"},
    props={"title": "String", "window": "(w,h)", "density": "Density", "on_back": "EventHandler<()>", "body": "Element", "footer": "Option<Element>", "hints": "Option<Element>", "keyboard_inset": "f32", "reveal": "Option<(top,bottom)>", "wide": "bool", "fixed": "bool", "back_focused": "bool"},
    note="See surfaces.fullScreenPage. Preview: components/FullScreenPage."), "name")
upsert(d["components"], comp("ModalMenu", "rect (Layer::Overlay) + ScrollView per column", "surface::ModalMenu", kind="surface",
    tokens=["deck-bg", "bg-panel", "ink", "ink-muted", "ink-subtle"],
    props={"levels": "Vec<MenuLevelView>", "placement": "Centered | Anchored{x,y}", "window": "(w,h)", "density": "Density", "on_pick": "EventHandler<(level,index)>", "on_dismiss": "EventHandler<()>", "ring_visible": "bool"},
    note="See surfaces.modalMenu. Preview: components/ModalMenu."), "name")
upsert(d["components"], comp("ConfirmOverlay", "rect (Layer::Overlay)", "deck::ConfirmOverlay", kind="deck",
    tokens=["bg-panel", "line", "danger"], props={"copy": "ConfirmCopy{title,message,action_label}", "focus": "FocusId", "window": "(w,h)"},
    note="Deck's confirmation: darkened screen, centered card, Cancel focused first so the destructive choice is never one stray press away."), "name")
upsert(d["components"], comp("SettingRow", "rect + Input", "surface::SettingRow", kind="surface",
    tokens=["bg-panel", "bg-raised", "accent", "ink", "ink-muted", "danger"], size={"height": "deck-settings-row-h (+72 text row)"},
    props={"label": "String", "description": "Option<String>", "control": "Toggle(bool) | Value{text,opens_menu} | Text{input,placeholder,a11y} | Action{danger}", "focused": "bool", "density": "Density"},
    note="One line of a settings page. Preview: components/SettingRow."), "name")
upsert(d["components"], comp("SettingsBody", "rect + RevealScroll", "deck::SettingsBody", kind="deck",
    props={"schema": "Schema", "target": "SettingsTarget", "values": "SettingsValues", "section": "usize", "two_pane": "bool", "drilled": "bool", "focus": "FocusId", "viewport": "f32", "reveal": "Option<(top,bottom)>"},
    note="See surfaces.settingsPage. Preview: components/SettingsPage."), "name")
upsert(d["components"], comp("DesktopApp", "App content", "desktop::DesktopApp", props={"host": "HostState", "layout": "Option<LayoutClass>", "density": "Option<Density>", "on_action": "Option<EventHandler<(id,MenuAction)>>", "on_deck_mode": "Option<EventHandler<()>>"},
    note="The desktop Library at the layout class and density the window calls for. Sets no theme; the root does."), "name")
upsert(d["components"], comp("Shell", "App content", "shell::Shell", props={"host": "HostState", "feed": "ActionFeed", "map": "ActionMap", "on_effect": "EventHandler<Effect>", "detected": "UiMode", "dev": "DevOverrides", "script": "Vec<Action>"},
    note="The root. See the `shell` section."), "name")
for c in d["components"]:
    if c["name"] == "DeckApp":
        c["props"] = {"host": "HostState", "feed": "ActionFeed", "on_effect": "EventHandler<Effect>", "map": "ActionMap", "script": "Vec<Action> (applied once at startup; effects dropped)"}
        c["note"] = ("Owns no data: reads HostState, reports Effects. Pages: Home, Game, Install (FullScreenPage), Settings (FullScreenPage). Overlays: SlidePanels, centered ModalMenu, ConfirmOverlay. "
                     "Provides ManagedFocus; the root is not a focus target while typing.")

# HintBar: leading/trailing groups
for c in d["components"]:
    if c["name"] == "HintBar":
        c["layout"] = "row; Menu and Quick access hints at the left edge, Options / Select / Back at the right (as in Big Picture); `tight` shrinks the gaps under 520px"

# ---- decisions
for e in [
    {"id": "surface-by-kind", "decision": "Every popup is one of three kinds (Form, Confirm, Menu) and one table decides how each is presented at each form factor.", "why": "Dialogs, settings and menus all needed the same answer to 'popup or whole screen?'; asking it in one place keeps the screens simple and the answer testable.",
     "rejected": "a per-screen `if phone` branch (drifts, untestable); making every dialog full-screen on touch (a confirmation would be all empty space)"},
    {"id": "form-fullscreen", "decision": "Forms are full-screen pages with Back on phones, controllers and short touch screens; popups stay on desktop and tall touch screens.", "why": "A popup leaves too little room for fields and a keyboard, and a pad cannot aim at small targets. The mobile settings-page pattern gives the fields the whole screen.",
     "rejected": "bottom sheets (Freya's Popup is always centered and a sheet needs a custom overlay; no gain over a page)"},
    {"id": "keyboard-reveal", "decision": "The host reports the keyboard height; the page hides its footer, adds a spacer and scrolls the focused field into the visible part.", "why": "Freya exposes no keyboard insets, so the host must supply them; hiding the footer gives a landscape phone back the space the keyboard took.",
     "rejected": "resizing the window (the OS does not on every platform); leaving the footer under the keyboard (hides the fields it controls)"},
    {"id": "menu-centered", "decision": "On touch and pad, option menus open centered over a darkened screen with the focused row inverted; on a pointer they are anchored popovers.", "why": "Matches Big Picture's menu behavior and reads at distance; a pointer user expects a context menu where they clicked."},
    {"id": "text-entry-replaces", "decision": "Entering a text box by pad or keyboard clears it (old text returns if nothing is typed).", "why": "Freya's Input cannot place its caret from outside, so typing would land before the old text.", "rejected": "a custom text box; selecting all (not exposed)"},
    {"id": "managed-focus", "decision": "Deck buttons are out of Freya's Tab order.", "why": "A button that held Freya focus fired on Enter as well as our Confirm."},
    {"id": "actions-in-header", "decision": "Under surface-short-h tall, a full-screen page puts its footer actions in the header.", "why": "A 480px landscape handheld cannot spare a header, a footer and a hint bar."},
    {"id": "files-small", "decision": "Files stay under 1000 lines and are split by responsibility; module roots document themselves; pure logic is Freya-free.", "why": "Small single-purpose files are cheaper for people and AI assistants to read, change and test; enforced by tests/repo_hygiene.rs."},
]:
    upsert(d["decisions"], e, "id")

d["notBuilt"] = [
    "OnScreenKeyboard (the OS provides one; HostState.keyboard_inset is how the UI learns its height; a simulated one is built into Shell for testing)",
    "FileBrowser (gamepad file picker); Effect::ChooseFile asks the host and the host answers through HostState.chosen_file",
    "Desktop Settings / Properties route (Properties exists as a Deck page; the desktop's Manage menu omits Properties)",
    "Catalog and Mods sections (empty state only)",
    "A caret-aware text box (Freya 0.5-rc Input cannot be told where its caret is)",
    "Detecting touch input to switch the desktop to Touch density automatically (today: RECLAW_DENSITY or the host sets it)",
    "Deck mode below about 520px wide is tolerable but not designed (tab strip text wraps)",
    "Virtual keyboard/mouse device (uinput) for apps that do not read SDL",
    "Bringing the launcher/app window forward (platform glue; the model emits Effects)",
    "Windows Guide-button capture and Windows job-object process control (taskkill fallback is untested)",
    "Hardware test of the gilrs backend",
    "Big Art from real catalog images and dominant-color extraction (Backdrop takes a tint and an optional image)",
]
d["verification"].update({
    "workspace": "cargo test --workspace: reclaw-input, reclaw-runtime, and reclaw-ui (unit tests beside the code; integration tests deck_input, deck_surfaces, deck_snapshots, desktop_surfaces, desktop_snapshots, shell_modes, deck_lifecycle, tokens_in_sync, repo_hygiene). ARCHITECTURE.md has the map.",
    "surfaces": "tests/deck_input.rs and tests/desktop_surfaces.rs assert, through the real component tree, popup vs full-screen per form factor, the footer hiding and the focused row scrolling above a keyboard (positions measured from layout), the cascading menu by key and by click, uninstall asking first, text entry, and Freya focus keys not double-firing.",
    "interfaceSwitching": "tests/shell_modes.rs: F10, the desktop button, the Deck main menu, the Interface setting, the pad working after leaving and re-entering Deck mode, F9 and the environment keyboard.",
    "hygiene": "tests/repo_hygiene.rs: no Rust file over 1000 lines, every module root documented, no file with more than 150 lines of inline tests.",
    "lint": "cargo clippy --workspace --all-targets --features reclaw-ui/gamepad is clean; cargo fmt --all --check is clean (rustfmt.toml).",
})
d["verification"]["notVerified"] = list(dict.fromkeys(d["verification"]["notVerified"] + [
    "on-screen keyboard behavior on a real Android, iOS or SteamOS touch device (the inset is simulated; no device was available)",
    "Freya's Input caret and focus behavior beyond what the headless runner shows",
]))
json.dump(d, open(P, "w"), indent=1)
print("contract updated:", len(d["components"]), "components,", len(d["decisions"]), "decisions")
