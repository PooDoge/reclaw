import os
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))  # work from design-system/, wherever this is run
import os
P="components"
FONT='<link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Hanken+Grotesk:wght@400;600;700&family=JetBrains+Mono:wght@400&display=swap">'
def preview(name,group,height,subtitle,body,pad=16,extra_css=""):
    marker=f'<!-- @dsCard group="{group}" height={height} subtitle="{subtitle}" -->'
    html=f'''{marker}
<!doctype html><html><head><meta charset="utf-8">{FONT}
<style>body{{margin:0;padding:{pad}px;background:var(--bg-base);font-family:var(--font-sans);color:var(--ink)}}.row{{display:flex;gap:12px;align-items:center;flex-wrap:wrap}}.col{{display:flex;flex-direction:column;gap:12px}}.cap{{font:700 11px/14px var(--font-sans);letter-spacing:.08em;text-transform:uppercase;color:var(--ink-subtle);margin:0 0 6px}}{extra_css}</style></head>
<body><div id="r"></div><script>
var R=window.Reclaw,h=React.createElement;
function App(){{return {body}}}
ReactDOM.createRoot(document.getElementById("r")).render(h(App));
</script></body></html>'''
    os.makedirs(f"{P}/{name}",exist_ok=True)
    open(f"{P}/{name}/preview.html","w").write(html)
def readme(name,text):
    open(f"{P}/{name}/README.md","w").write(text.strip()+"\n")

preview("Button","Actions",150,"Install, primary, secondary, ghost, danger; pointer and touch sizes",
'''h("div",{className:"col"},
 h("div",{className:"row"},h(R.Button,{variant:"install",size:"lg",icon:"play"},"Play"),h(R.Button,{variant:"install",size:"lg",icon:"download"},"Install"),h(R.Button,{variant:"primary"},"Check for updates"),h(R.Button,null,"Manage"),h(R.Button,{variant:"ghost",icon:"folder"},"Open folder"),h(R.Button,{variant:"danger"},"Uninstall")),
 h("div",{className:"row"},h(R.Button,{variant:"install",state:"hover"},"Hover"),h(R.Button,{variant:"primary",state:"focus"},"Focus"),h(R.Button,{variant:"install",disabled:true},"Disabled"),h(R.Button,{variant:"install",size:"touch",icon:"download"},"Touch 48px")))''')
readme("Button","""
The call-to-action control. Freya: `Button::new().filled()` (install, primary), `.outline()` (secondary), `.flat()` (ghost), themed through `theme_colors(ButtonColorsThemePartial{..})`.

Use `install` for the one verb that changes the game's state (Install, Play, Update); one per screen region, never two side by side. Use `primary` for non-game actions such as Check for updates. Labels are verbs in sentence case.

Consumer provides: label child, optional Lucide icon, `on_press`. Pointer height 32 (44 for the hero action); at touch density use `size: touch`, 48px tall, never below `target-min`.

Don't: color a button by game status; use `danger` anywhere but a confirmed destructive step; disable the install button without saying why in an adjacent badge.
""")

preview("Chip","Actions",90,"Tag filters, selected and with counts",
'''h("div",{className:"row"},h(R.Chip,{selected:true,count:24},"All"),h(R.Chip,{count:9},"Installed"),h(R.Chip,{count:3},"Updates"),h(R.Chip,{count:5},"N64"),h(R.Chip,{count:2},"Mods"),h(R.Chip,null,"Favorites"))''')
readme("Chip","""
Tag and status filter, carried over from Quiver's tag filters. Freya: `Chip::new().selected(..).on_press(..)`.

Wrap onto a second row on narrow widths; never scroll horizontally on phone. The count is mono and optional. Selected uses the accent fill with `on-accent` text; unselected shows a `line-strong` border so it is visible without relying on the fill.
""")

preview("StatusBadge","Status",110,"Installed, update ready, installing, failed, not installed, needs your game file",
'''h("div",{className:"row"},["installed","update","installing","failed","available","needsfile"].map(function(k){return h(R.StatusBadge,{key:k,kind:k})}))''')
readme("StatusBadge","""
One badge per game state. Every badge carries a word and, except Not installed, an icon, so state never depends on hue: the green and red here differ in lightness and by label.

States: `installed` (ok), `update` (warn), `installing` (info), `failed` (danger), `available` (neutral), `needsfile` (warn). **Needs your game file** is specific to recompilation: the project ships no copyrighted assets, so the user must supply their own ROM or disc image before Install can finish. Never offer to download it.

Freya: a custom `rect()` with a Lucide icon and a `label()`; fill and text come from the `*-bg` and matching status token.
""")

preview("Switch","Inputs",80,"On, off, disabled",
'''h("div",{className:"row"},h(R.Switch,{on:true}),h(R.Switch,{on:false}),h(R.Switch,{on:true,disabled:true}))''')
readme("Switch","""
Binary setting (shortcuts, pre-releases, auto-update). Freya: `Switch::new().toggled(..).on_toggle(..)`. Always pair with a text label to its left; the whole row is the hit target at touch density. Off state keeps a `line-strong` border (3:1).
""")

preview("SearchField","Inputs",120,"Placeholder, focused with value, touch size",
'''h("div",{className:"col",style:{maxWidth:420}},h(R.SearchField,null),h(R.SearchField,{value:"n64 mods",focus:true}),h(R.SearchField,{size:"touch"}))''')
readme("SearchField","""
Library and catalog filter (name, tag, repo, folder, as in Quiver). Freya: `Input::new(writable).placeholder(..)`; give it an explicit `.background(bg_raised)` or it renders invisible. Focus draws a solid 2px accent border. On phone use the touch size and let it fill the width.
""")

preview("GameCapsule","Library",340,"Resting, hover with Play, selected; art is a placeholder",
'''h("div",{className:"row",style:{alignItems:"flex-start",gap:16}},h(R.GameCapsule,{title:"Starfall 64",project:"N64Recomp",status:"installed"}),h(R.GameCapsule,{title:"Starfall 64",project:"N64Recomp",status:"installed",hover:true}),h(R.GameCapsule,{title:"Kart Ruins",project:"N64Recomp",status:"needsfile",selected:true}),h(R.GameCapsule,{title:"Dino Rush",project:"PS2 recomp",status:"available"}))''',extra_css="")
readme("GameCapsule","""
The Steam-style library tile: 3:4 portrait art, title, recompilation project, status badge. Freya: a `Card` with fixed width `capsule-w` (168), image via `image()`; `Content::Flex` is not needed because the card is a plain column.

Consumer provides: art (catalog supplies it; fall back to the striped placeholder with the title set in mono, never a generated logo), title, project, status. Hover lifts 2px, draws an accent border and, if installed, reveals the Play button over the art; on touch there is no hover, so Play appears in the hero, not the tile.

Grid: columns `repeat(auto-fill, capsule-w)` on wide, 4 fluid columns on compact, 2 on phone with `fluid` (width 100%).
""")

preview("LibraryRow","Library",260,"Sidebar list rows: resting, hover, selected, each status dot",
'''h("div",{className:"row",style:{alignItems:"flex-start",gap:24}},h("div",{className:"col",style:{width:240,gap:2}},h(R.LibraryRow,{title:"Starfall 64",status:"installed",selected:true,version:"v1.4.2"}),h(R.LibraryRow,{title:"Skyward Quest",status:"update",hover:true}),h(R.LibraryRow,{title:"Kart Ruins",status:"needsfile"}),h(R.LibraryRow,{title:"Dino Rush",status:"available"}),h(R.LibraryRow,{title:"Moon Garden",status:"failed"})),h("div",{className:"col",style:{width:300,gap:2}},h("p",{className:"cap"},"Touch density"),h(R.LibraryRow,{title:"Starfall 64",status:"installed",selected:true,size:"touch"}),h(R.LibraryRow,{title:"Skyward Quest",status:"update",size:"touch"})))''')
readme("LibraryRow","""
Left-rail library entry (Steam's game list). Freya: `SideBarItem::new().on_press(..)` with active state from `use_is_active()`; the row is `rect().horizontal().content(Content::Flex)` with the name as `Size::flex(1.)`, then version and status dot at fixed size (the Content::Flex rule applies).

Selected: `bg-raised` fill plus a 3px accent bar on the left, so selection is not carried by fill alone. The dot is a secondary cue; the state's word appears in the hero's badge. Pointer height 32, touch 48.
""")

preview("HeroHeader","Library",470,"Game page banner with action bar: wide and narrow",
'''h("div",{className:"col",style:{gap:24}},h(R.HeroHeader,{title:"Starfall 64",project:"N64Recomp",version:"v1.4.2",status:"installed"}),h("div",{style:{maxWidth:390}},h(R.HeroHeader,{title:"Skyward Quest",project:"Zelda-style port",version:"v0.9.1",status:"update",narrow:true,source:"gitlab.com"})))''')
readme("HeroHeader","""
Game page top: 16:5 banner, project eyebrow in accent, title, and an action bar holding the single install verb, version, source and secondary actions. Freya: `rect()` with `image()` background, a flex action row (`Content::Flex`, spacer as `Size::flex(1.)`).

Wide: title `title-hero`, bar padding `space-6`. Narrow (compact/phone): `title-page`, bar padding `space-4`, secondary actions collapse to icon-only ghost buttons and the bar wraps. The title sits on a solid `bg-base` strip over the art so it holds 4.5:1 on any banner image.
""")

preview("DownloadItem","Progress",280,"Queue entries: fetching, verifying, failed",
'''h("div",{className:"col",style:{maxWidth:620}},h(R.DownloadItem,{title:"Starfall 64  v1.4.3",stage:"Fetching release",value:34,detail:"21 MB of 61 MB",speed:"7.4 MB/s"}),h(R.DownloadItem,{title:"Skyward Quest  v0.9.2",stage:"Verifying hash",value:100,detail:"Checking sha256",speed:"",tone:"ok"}),h(R.DownloadItem,{title:"Moon Garden  v2.0.1",stage:"Failed",value:48,detail:"Release asset not found",speed:"",tone:"danger"}),h(R.ProgressBar,{value:62}))''')
readme("DownloadItem","""
Download queue row (Steam's Downloads page). The stage label is mono and tells the recompilation pipeline's step: Fetching release, Verifying hash, Checking your game file, Building, Extracting. Freya: `ProgressBar` inside a flex column; the progress fill is accent (info), switching to ok at 100% verified and danger on failure; failure always shows the reason in the detail line, never just a red bar.

Cancel is an icon ghost button with an accessible label.
""")

preview("Nav","Navigation",420,"Top bar (wide), icon rail (compact), bottom tabs (phone)",
'''h("div",{className:"col",style:{gap:20}},h("div",null,h("p",{className:"cap"},"top, at least 1100px"),h(R.Nav,{mode:"top",active:"library"})),h("div",{className:"row",style:{alignItems:"flex-start",gap:32}},h("div",null,h("p",{className:"cap"},"rail, 720 to 1099px"),h("div",{style:{height:240,display:"flex"}},h(R.Nav,{mode:"rail",active:"catalog",style:{height:"100%"}}))),h("div",{style:{width:360}},h("p",{className:"cap"},"bottom tabs, under 720px"),h(R.Nav,{mode:"bottom",active:"queue"}))))''')
readme("Nav","""
Primary navigation: Library, Catalog (community app lists, from Quiver), Downloads, Mods (Thunderstore, GameBanana). Three forms of one component, chosen by the container width, not the OS:

- `top` at ≥1100px: Steam-style uppercase eyebrow items with a 2px accent underline on the active one.
- `rail` at 720-1099px: 64px icon rail, 44px hit targets, 3px accent bar on active.
- `bottom` under 720px: 56px tab bar, icon over 11px label, 44px minimum targets, active in accent.

Freya: `rect()` rows/columns of `SideBarItem`/`FloatingTab`; the icon-only rail needs a `TooltipContainer` per item. Downloads shows a count badge while the queue is active.
""")

preview("InstallDialog","Dialogs",520,"Install location and the user's own game file",
'''h("div",{className:"col"},h(R.InstallDialog,{title:"Starfall 64",slug:"starfall-64",file:""}))''',pad=0)
readme("InstallDialog","""
Modal shown from the hero's Install. Freya: `Popup::new()` with `PopupTitle`, `PopupContent`, `PopupButtons`; backdrop is `scrim`.

Install stays disabled until the user's own game file is chosen, and the field label says it is never downloaded for you. On phone the dialog becomes a bottom sheet (full width, `radius-lg` on top corners only, 300ms slide), with 48px fields and buttons.
""")

# Screens page
preview("Screens","Screens",1750,"Library at wide, compact and phone widths",
'''h("div",{className:"col",style:{gap:32,alignItems:"flex-start"}},[["wide","Wide, at least 1100px: top nav, library sidebar, hero"],["compact","Compact, 720 to 1099px: icon rail, hero, capsule grid"],["phone","Phone / handheld, under 720px: bottom tabs, 2-column grid, 48px targets"]].map(function(s){var f=s[0]==="wide"?1100:s[0]==="compact"?860:390;return h("div",{key:s[0],style:{zoom:Math.min(1,(window.innerWidth-32)/f)}},h("p",{className:"cap"},s[1]),h(R.LibraryScreen,{mode:s[0]}))}))''')
readme("Screens","""
Reference compositions of the Library page at the three layout classes. Widths are the **container** width (Freya: `on_sized` on the root, guarded with an epsilon), so the same code serves a desktop window, a handheld at 1280x800 and a phone. Density (pointer vs touch/controller) is a separate switch from width: a 1280px handheld uses the wide layout with touch row heights and 48px buttons.
""")
# fix marker for page
p=f"{P}/Screens/preview.html"; s=open(p).read().replace('subtitle="Library at wide, compact and phone widths"','page subtitle="Library at wide, compact and phone widths"'); open(p,"w").write(s)
