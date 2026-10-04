"""Adds the surface components (full-screen page, centered menu, settings page) to the design system:
patches components/bundle.{js,css} between marker comments and writes their README + preview files.
Safe to run again. Run from anywhere: `python3 design-system/tools/gen_surfaces.py`."""
import os, re
exec(open("tools/gen_components.py").read().split('preview("Button"')[0])

NAMES = ["SettingRow", "FullScreenPage", "ModalMenu", "ConfirmCard", "SettingsNav"]
OWNED = set(NAMES) | {"SettingsPage"}  # names this script has ever put in the manifest

def patch(path, start, end, block):
    s = open(path).read()
    pat = re.compile(re.escape(start) + r".*?" + re.escape(end) + r"\n?", re.S)
    s = pat.sub("", s).rstrip("\n") + "\n" + start + "\n" + block.rstrip("\n") + "\n" + end + "\n"
    open(path, "w").write(s)

patch("components/bundle.css", "/* surfaces:start */", "/* surfaces:end */", open("tools/surfaces.css").read())
patch("components/bundle.js", "/* surfaces:start */", "/* surfaces:end */", open("tools/surfaces.js").read())
js = open("components/bundle.js").read()
first, rest = js.split("\n", 1)
m = re.match(r"(/\* @ds-bundle: )(\{.*\})( \*/)", first)
import json
manifest = json.loads(m.group(2))
manifest["components"] = [c for c in manifest["components"] if c["name"] not in OWNED]
have = {c["name"] for c in manifest["components"]}
for n in NAMES:
    if n not in have:
        manifest["components"].append({"name": n})
open("components/bundle.js", "w").write(m.group(1) + json.dumps(manifest, separators=(",", ":")) + m.group(3) + "\n" + rest)

def deck_preview(name, height, subtitle, body):
    preview(name, "Surfaces", height, subtitle, body, pad=24, extra_css="body{background:var(--deck-bg)}")
    p = f"{P}/{name}/preview.html"; s = open(p).read()
    # Deck follows the theme (ADR 0007): the preview shows whichever theme the host renders.
    open(p, "w").write(s)

def readme(name, text):
    os.makedirs(f"{P}/{name}", exist_ok=True)
    open(f"{P}/{name}/README.md", "w").write(text.strip() + "\n")

HINTS = '[["confirm","Select"],["back","Back"]]'
ACTIONS = '[h(R.Button,{key:1,variant:"secondary",size:"lg"},"Cancel"),h(R.Button,{key:2,variant:"install",size:"lg",icon:"download"},"Install")]'

def install_rows(kb):
    return ('[h(R.SettingRow,{key:1,label:"Install location",control:"text",value:"~/Reclaw/Apps",focus:true,typing:%s}),'
            'h(R.SettingRow,{key:2,label:"Your game file",desc:"Never downloaded for you; Reclaw builds from your copy.",control:"info",value:"Choose file"}),'
            'h(R.SettingRow,{key:3,label:"Create desktop shortcut",control:"toggle",on:true}),'
            'h(R.SettingRow,{key:4,label:"Keep pre-release builds",control:"toggle"})]' % ("true" if kb else "false"))

def frame(label, w, hh, scale, page):
    return (f'h("div",null,h("div",{{className:"sf-cap"}},"{label}"),h(R.Scaled,{{w:{w},h:{hh},scale:{scale}}},{page}))')

def page(w, hh, title, rows, kb=0, deck=True, hints=True, actions=True, wide=False, fixed=False):
    return (f'h(R.FullScreenPage,{{w:{w},h:{hh},title:{title!r},deck:{str(deck).lower()},keyboard:{kb},wide:{str(wide).lower()},fixed:{str(fixed).lower()},'
            f'actions:{ACTIONS if actions else "null"},hints:{("h(R.HintBar,{hints:" + HINTS + "})") if hints else "null"}}},{rows})')

deck_preview("FullScreenPage", 520, "One page for every form: Back, scrolling body, footer actions; the short and keyboard-up variants",
 'h("div",{style:{display:"flex",gap:24,flexWrap:"wrap",alignItems:"flex-start"}},'
 + frame("Handheld or TV, 1280x800", 1280, 800, .45, page(1280, 800, "Install Dino Rush", install_rows(False))) + ","
 + frame("Landscape phone, 854x480: actions move to the header", 854, 480, .6, page(854, 480, "Install Dino Rush", install_rows(False))) + ","
 + frame("Keyboard up, 854x480: footer and hints hide, field stays above the keyboard", 854, 480, .6, page(854, 480, "Install Dino Rush", install_rows(True), kb=216)) + ","
 + frame("Portrait phone, 390x844", 390, 844, .42, page(390, 844, "Install Dino Rush", install_rows(False))) + ")")

readme("FullScreenPage", '''
The page every form, settings screen and install flow becomes on a phone, a short touch screen and in Deck mode, in the pattern of a mobile settings page. Rust: `reclaw_ui::surface::FullScreenPage`; `Dialog` chooses it for you.

**Anatomy.** Header (`surface-header-h` 64, Back button + title), body, footer (`surface-footer-h` 88, actions right-aligned), hint bar in Deck mode. The body is a centered column no wider than `surface-max-w` (720); two-pane pages use the full width.

**When it is used** (`surface::presentation`): Form surfaces use it when the layout class is Phone, the density is Controller, or the density is Touch and the window is under `surface-touch-popup-min-h` (900) tall. Otherwise they are a popup.

**Short windows.** Under `surface-short-h` (600) tall, the footer's actions move into the header and the footer row disappears, so a 480px landscape handheld keeps most of its height for the body.

**Keyboard avoidance.** The host reports the on-screen keyboard height (`HostState::keyboard_inset`). While it is above zero: the footer and hint bar hide, the body ends in a spacer as tall as the keyboard so the last field can scroll clear, the header shrinks to `surface-header-h-compact` (48) if the keyboard covers more than 40% of the window, and the focused field is scrolled into the part of the window still visible (`scroll_to_reveal`, 16px margin). Back stays reachable.

**Back** is the header button, the B button and Esc; all three do the same thing.
''')

# ModalMenu
UNDER = 'h(R.DeckScreen,{})'
def menu(title, cols, w=1280, hh=800, scale=.5, colw=None, under=UNDER):
    cw = f",colw:{colw}" if colw else ""
    return f'h(R.ModalMenu,{{w:{w},h:{hh},scale:{scale},title:{title!r},under:{under},columns:{cols}{cw}}})'

root_rows = '[["Add to favorites",{focus:true}],["Add to",{chev:true}],["Manage",{chev:true}],["Properties...",{sep:true}],["Cancel",{sep:true}]]'
casc_left = '[["Add to favorites"],["Add to",{chev:true,open:true}],["Manage",{chev:true}],["Properties...",{sep:true}],["Cancel",{sep:true}]]'
casc_right = '[["Handheld friendly",{focus:true}],["Backlog"],["Recompiled N64"],["New collection...",{sep:true}]]'
manage_rows = '[["Open install folder",{focus:true}],["Verify files"],["Check for updates"],["Uninstall",{sep:true}]]'
deck_preview("ModalMenu", 520, "Options menu over a darkened screen: root, cascading submenu, a narrow window, and the confirmation it can lead to",
 'h("div",{style:{display:"flex",gap:24,flexWrap:"wrap",alignItems:"flex-start"}},'
 + frame("Root", 1280, 800, .42, menu("Starfall 64", f"[{root_rows}]", scale=.4)) + ","
 + frame("Submenu opens to the right; the parent keeps its open row marked", 1280, 800, .4, menu("Add to", f"[{casc_left},{casc_right}]", scale=.4)) + ","
 + frame("Narrow window: only the deepest level", 390, 844, .42, menu("Manage", f"[{manage_rows}]", 390, 844, .42, colw=342, under='null')) + ","
 + 'h("div",null,h("div",{className:"sf-cap"},"Confirmation (Cancel has focus first)"),h("div",{style:{padding:24,background:"rgba(5,8,12,.85)"}},h(R.ConfirmCard,{title:"Uninstall Starfall 64?",message:"Removes it from this device. Your own game file is never touched.",action:"Uninstall"}))))')

readme("ModalMenu", '''
The Options menu, in the shape of Big Picture's context menu. Rust: `reclaw_ui::surface::{ModalMenu, MenuState}`; the entries for an app are `app_menu::options_menu`.

**Centered** (touch, pad, phones): the screen darkens to a 85% scrim, the title sits above, the menu is centered. Rows are `deck-row-h` tall in columns `deck-menu-w` wide. The focused row is **inverted** (`ink` fill, `deck-bg` text), which reads at ten feet. `>` rows open a submenu in a second column to the right; the parent stays visible with its open row in `ink-muted`. Group separators are 4px gaps of `deck-bg`. Disabled rows use `ink-subtle`. When the window cannot fit two columns only the deepest level shows, and Back returns one level.

**Anchored** (pointer, desktop): a popover at the press point with a border and shadow, no scrim, 40px rows, 280px wide; it slides to stay on screen. A click outside dismisses it.

Behavior lives in `MenuState` (focus per level, open path, Back), tested without a window: Up and Down stop at the ends and skip disabled rows; Right or Confirm on `>` opens it; Left or Back closes one level; Back at the root closes the menu. Pickers behind a choice row use the same menu without a Cancel row.
''')

# SettingsPage
rows_iface = ('[h(R.SettingRow,{key:1,label:"Interface",desc:"Auto picks Deck mode under a console session; the others force one.",control:"value",value:"Auto",focus:true}),'
              'h(R.SettingRow,{key:2,label:"UI scale",control:"value",value:"100%"}),'
              'h(R.SettingRow,{key:3,label:"Screen edge margin",desc:"Raise it if the edges of your TV are cut off.",control:"value",value:"Small"})]')
two_pane = ('h("div",{style:{display:"flex",height:"100%"}},h(R.SettingsNav,{items:["Interface","Controller","Library","About"],sel:0}),h("div",{className:"sf-rows"},' + rows_iface + '))')
list_view = 'h(R.SettingsNav,{full:true,items:["Interface","Controller","Library","About"],sel:0,focus:0})'
drilled = ('h("div",{className:"sf-col"},h(R.SettingRow,{label:"Check for updates when Reclaw starts",control:"toggle",on:true}),'
           'h("div",{className:"sf-heading"},"Install location"),h(R.SettingRow,{label:"Default install location",control:"text",value:"~/Reclaw/Apps",focus:true}))')
def settings_page(w, hh, title, body, fixed=False, wide=False):
    return (f'h(R.FullScreenPage,{{w:{w},h:{hh},title:{title!r},deck:true,fixed:{str(fixed).lower()},wide:{str(wide).lower()},actions:null,'
            f'hints:h(R.HintBar,{{hints:[["menu","Menu"],["confirm","Select"],["back","Back"]]}})}},{body})')
deck_preview("SettingsPage", 520, "Settings and Properties: two panes on a wide window, a list that drills into a section on a narrow one",
 'h("div",{style:{display:"flex",gap:24,flexWrap:"wrap",alignItems:"flex-start"}},'
 + frame("Two panes, 900px and wider", 1280, 800, .45, settings_page(1280, 800, "Settings", two_pane, fixed=True, wide=True)) + ","
 + frame("Narrow: sections list", 390, 844, .42, settings_page(390, 844, "Settings", list_view)) + ","
 + frame("Narrow: one section's rows (Back returns to the list)", 390, 844, .42, settings_page(390, 844, "Library", drilled)) + ")")

readme("SettingsPage", '''
Reclaw's Settings and an app's Properties are the same page built from a schema (`deck::settings`: sections, groups, rows). Rust: `reclaw_ui::deck::SettingsBody` inside a `FullScreenPage`.

**Layout.** At `deck-two-pane-min-w` (900) and wider: a section list `deck-settings-nav-w` (400) wide beside the rows, each scrolling on its own. Narrower: the section list alone, then a section's rows after a press; Back goes one step back, then leaves. The selected section has a blue wash, the focused one a raised fill with an accent bar.

**Rows** (`SettingRow`): `deck-settings-row-h` (68) tall, one line of label and one of description (truncated, so keep descriptions short). A toggle shows a switch, a choice shows its value and a chevron and opens a centered menu, info shows plain text, an action is the whole row (danger in red). A text row is 72px taller and holds an input: pressing it from a pad or keyboard starts typing, clearing the box (the old text returns if nothing is typed) because Freya's input cannot move its caret from outside; a tap or click keeps the text.

Layout and focus both read `section_slots`, so what is drawn and what can be focused cannot drift apart.
''')

readme("SettingRow", '''
One line of a settings page. Rust: `reclaw_ui::surface::SettingRow`. See `SettingsPage` for the page; `InstallBody` uses the same rows for the install form.

Controls: `toggle`, `value` (a choice: opens a menu), `info` (read only), `text` (an input under the label, 72px taller), `action` (the row is the button). Focus draws a raised fill and a `deck-focus-ring` accent border.
''')
deck_preview("SettingRow", 480, "Toggle, choice, info and text rows; one focused",
 'h("div",{style:{display:"flex",flexDirection:"column",gap:8,width:760}},'
 'h(R.SettingRow,{label:"Create desktop shortcut",control:"toggle",on:true,focus:true}),'
 'h(R.SettingRow,{label:"Interface",desc:"Auto picks Deck mode under a console session.",control:"value",value:"Auto"}),'
 'h(R.SettingRow,{label:"Version",control:"info",value:"v1.4.2"}),'
 'h(R.SettingRow,{label:"Launch options",control:"text",value:"",placeholder:"Arguments passed to the app"}),'
 'h(R.SettingRow,{label:"Uninstall",control:"action",danger:true}))')
os.makedirs(f"{P}/SettingRow", exist_ok=True)
print("surfaces written:", ", ".join(NAMES))
