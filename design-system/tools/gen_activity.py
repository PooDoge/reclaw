"""Adds the components of background activity, notices, systems, media and the window frame to the design system:
patches components/bundle.{js,css} between marker comments and writes their README + preview files.
Safe to run again. Run from anywhere: `python3 design-system/tools/gen_activity.py`."""
import os, re, json
exec(open("tools/gen_components.py").read().split('preview("Button"')[0])

NAMES = ["SystemBadge", "BadgeChip", "IndicatorBadge", "CardIndicator", "HoldRing", "NoticeToast", "Titlebar", "RemoteArt", "BannerArt"]

def patch(path, start, end, block):
    s = open(path).read()
    pat = re.compile(re.escape(start) + r".*?" + re.escape(end) + r"\n?", re.S)
    s = pat.sub("", s).rstrip("\n") + "\n" + start + "\n" + block.rstrip("\n") + "\n" + end + "\n"
    open(path, "w").write(s)

patch("components/bundle.css", "/* activity:start */", "/* activity:end */", open("tools/activity.css").read())
patch("components/bundle.js", "/* activity:start */", "/* activity:end */", open("tools/activity.js").read())
js = open("components/bundle.js").read()
first, rest = js.split("\n", 1)
m = re.match(r"(/\* @ds-bundle: )(\{.*\})( \*/)", first)
manifest = json.loads(m.group(2))
manifest["components"] = [c for c in manifest["components"] if c["name"] not in NAMES]
manifest["components"] += [{"name": n} for n in NAMES]
open("components/bundle.js", "w").write(m.group(1) + json.dumps(manifest, separators=(",", ":")) + m.group(3) + "\n" + rest)

def readme(name, text):
    os.makedirs(f"{P}/{name}", exist_ok=True)
    open(f"{P}/{name}/README.md", "w").write(text.strip() + "\n")

def card(group, name, height, subtitle, body, deck=False):
    preview(name, group, height, subtitle, body, pad=24 if deck else 16, extra_css="body{background:var(--deck-bg)}" if deck else "")
    if deck:
        p = f"{P}/{name}/preview.html"; s = open(p).read()
        open(p, "w").write(s.replace('<div id="r"></div>', '<div id="r" data-theme="midnight"></div>'))

# ---- SystemBadge
card("Library", "SystemBadge", 150, "The system a game was recompiled from; neutral on purpose",
 '''h("div",{className:"col"},
  h("div",{className:"row"},["N64","PS1","PS2","GC","Wii","SNES","GBA","Xbox","X360","DC"].map(function(s){return h(R.SystemBadge,{key:s,short:s})})),
  h("div",{className:"row",style:{background:"linear-gradient(135deg,var(--info-bg),var(--bg-raised))",padding:12}},h(R.SystemBadge,{short:"N64",overArt:true}),h(R.SystemBadge,{short:"PS2",overArt:true})),
  h("div",{className:"row"},h(R.SystemBadge,{short:"N64",large:true}),h(R.SystemBadge,{short:"GC",large:true})))''')
readme("SystemBadge", """
The mark of the system a game was recompiled from ("N64", "PS2"), as a small chip. Rust: `reclaw_ui::components::SystemBadge`; names, order and grouping come from `reclaw_games::project::Platform` (one table, tested), the filter and sort by system from `reclaw_ui::systems`.

Neutral in color on purpose, like the controller glyphs: the letters carry the meaning, so it reads the same in both themes and for everyone, and it never competes with a status color. A 1px `line-strong` border, `ink-muted` mono text; `over_art` gives it a solid `bg-base` fill and `ink` text so it stays legible on any picture; `large` is the Deck size (hint-size type). A game of no known system draws nothing.

Where it shows: Library rows, capsules, the hero, catalog cards, Deck tiles (large, over art), and the system filter's picker. Deck's Home can group shelves by system (Settings > Library > Sort).

Don't: color it by system, or use it for status.
""")

# ---- BadgeChip
card("Media", "BadgeChip", 130, "A README's status badge drawn natively: label on grey, message on the badge's color",
 '''h("div",{className:"col"},
  h("div",{className:"row"},h(R.BadgeChip,{label:"build",message:"passing",color:"#2ea043"}),h(R.BadgeChip,{label:"license",message:"MIT",color:"#3b82c4"}),h(R.BadgeChip,{label:"release",message:"v1.4.2",color:"#e0a526",darkText:true}),h(R.BadgeChip,{label:"coverage",message:"61%",color:"#d4392b"})),
  h("div",{className:"row"},h(R.BadgeChip,{message:"no label",color:"#6e7681"})))''')
readme("BadgeChip", """
A README's status badge (shields-style) as two parts: the label on `bg-raised`, the message on the badge's own color. Rust: `reclaw_ui::readme::BadgeChip`; the parsing of the badge's address into label, message and color is `reclaw_media::readme` (pure, tested).

Drawn natively because the toolkit's SVG drawing has no fonts, so a fetched badge image would show without its text. Message text is white, or near-black when the color is light (`wants_dark_text`). Pressable when the README wrapped the badge in a link; the link opens in the system browser, never inside the app.

Only badges whose address says its label, message and color are drawn this way. A badge that doesn't (a dynamic one) is a link to its address.
""")

# ---- CardIndicator + IndicatorBadge
rows = [("update", "Update ready", None), ("queued", "Queued", None), ("downloading", "34%", .34), ("installing", "Finishing", None), ("done", "Updated", 1), ("failed", "Failed", None), ("mods", "+2 mods", None)]
cards = ",".join(f'h("div",{{key:{i},style:{{display:"flex",flexDirection:"column",gap:8}}}},h(R.CardIndicator,{{kind:"{k}",label:"{l}",progress:{"null" if p is None else p}}}),h("span",{{className:"ac-cap"}},"{k}"))' for i, (k, l, p) in enumerate(rows))
card("Activity", "CardIndicator", 620, "What a Deck card shows while its game has background work: an icon chip and a bar along the bottom edge",
 f'''h("div",{{style:{{display:"flex",flexWrap:"wrap",gap:24}}}},{cards})''', deck=True)
card("Activity", "IndicatorBadge", 120, "The small form, for rows and the desktop sidebar",
 '''h("div",{className:"row"},[["update","Update ready"],["queued","Queued"],["downloading","34%"],["installing","Finishing"],["done","Updated"],["failed","Failed"],["mods","+2 mods"]].map(function(x){return h(R.IndicatorBadge,{key:x[0],kind:x[0],label:x[1]})}))''')
readme("IndicatorBadge", """
The icon and short text of a game's background work: "34%", "Finishing", "Updated", "Update ready". Rust: `reclaw_ui::components::IndicatorBadge`; icon and color per kind in `indicator_look` (below); what a game's indicator is comes from `reclaw_ui::activity::indicator_for`, which is pure and tested.

| Kind | Icon (Lucide) | Color | Meaning |
|---|---|---|---|
| UpdateAvailable | download | `warn` | A newer version exists; nothing has started |
| Queued | clock | `ink-muted` | Waiting for its turn |
| Downloading | arrow-down-to-line | `accent` | Bytes are arriving |
| Installing | package | `info` | Verifying, extracting or finishing |
| Done | check | `ok` | Finished this run; stays until the app restarts, with what changed |
| Failed | triangle-alert | `danger` | Stopped; the notice says why |
| Mods | puzzle | `accent` | Only mods are downloading for this game |

Never by color alone: every kind has its own icon and a word or a number. In the desktop sidebar the Updates section lists the games that have one of these, with the same icons, and finished updates keep their row, with the changelog, until Reclaw restarts.
""")
readme("CardIndicator", """
What a Deck card shows over its art while its game has background work. Rust: `reclaw_ui::deck::CardIndicator` (in `deck/widgets/card_progress.rs`), built from the game's `Indicator`.

* A chip at the top left with the icon and text (large type, solid `bg-base` fill), and a bar along the bottom edge (`CARD_STRIP_H` = 8, a Rust constant, not yet a token).
* The bar **fills** when the size is known, **slides** (a third of the track, 1.4s, in-out) when it is not, is **full and green** when finished, and is **absent** when only an update is waiting or the job failed.
* Both sit inside the art's clip, so they scale with the focused card and keep its rounded corners.
* Only cards that need the sliding animation run it; a finished card is static.

Reduced motion: the sliding bar should become a static half-filled bar. Not implemented yet.
""")

# ---- NoticeToast + HoldRing
card("Activity", "NoticeToast", 560, "The newest notification, with the two press-and-hold prompts; the ring fills while the button is held",
 '''h("div",{style:{display:"flex",flexWrap:"wrap",gap:24,alignItems:"flex-start"}},
  h("div",null,h("div",{className:"ac-cap"},"At rest, Xbox"),h(R.NoticeToast,{kind:"xbox",tone:"update",title:"Update ready: Starfall 64",body:"Version 1.4.3 is ready to install."})),
  h("div",null,h("div",{className:"ac-cap"},"Holding X, 45%"),h(R.NoticeToast,{kind:"xbox",tone:"update",title:"Update ready: Starfall 64",body:"Version 1.4.3 is ready to install.",holding:"x",progress:.45})),
  h("div",null,h("div",{className:"ac-cap"},"Holding Y, PlayStation, 80%"),h(R.NoticeToast,{kind:"playstation",tone:"finished",title:"Starfall 64 updated",body:"Version 1.4.3. Hold for what changed.",holding:"y",progress:.8})),
  h("div",null,h("div",{className:"ac-cap"},"Keyboard (X and Y keys), failed"),h(R.NoticeToast,{kind:"keyboard",tone:"failed",title:"Download failed: Kart Ruins",body:"The server stopped answering.",holding:"x",progress:.2})))''', deck=True)
readme("NoticeToast", """
The newest notification as a card at the bottom right of Deck mode, with the two holds that act on it. Rust: `reclaw_ui::deck::NoticeToast` (`deck/widgets/notice.rs`); the queue and its rules are `reclaw_ui::notices` (a repeat replaces, the newest is shown, dismiss one or all).

* **Hold X** (900 ms) opens the notice's details; **hold Y** (1200 ms) dismisses every notice. The destructive one is longer. Durations are `DETAILS_AFTER` and `DISMISS_ALL_AFTER` in `notices/hold.rs`.
* The button's glyph sits inside a ring (`HoldRing`: 44px, a 4px `line-strong` track and an `accent` arc from the top, clockwise). The ring runs on its own animation clock for the hold's length; the pad reader's clock decides when the hold completes, and the two agree to within a frame.
* A tap still does what the button always does (X is Options, Y is Search); while a toast is up a tap is decided when the button comes up, so the hold can be told apart; a hold completes at its time, while the button is still down. The toast is **not focusable**: it never takes the pad from the page.
* Tone: update available `warn` + download; finished `ok` + check; failed `danger` + triangle-alert. The same icon and color table as the card indicator.
* Slides in from the right (280ms, expo-out) when a new notice arrives. The toast is 460 x 132 (Rust constants `TOAST_W`, `TOAST_H`; not yet tokens).

Unverified: the hold times on a real controller. They are untested outside the keyboard stand-in and the pad reader's tests.
""")

# ---- Titlebar
card("Window", "Titlebar", 330, "Our own title bar on a borderless window: drag area, minimize, maximize or restore, close",
 '''h("div",{className:"col"},
  h("div",null,h("div",{className:"ac-cap"},"Rest"),h(R.Titlebar,{page:"Library"})),
  h("div",null,h("div",{className:"ac-cap"},"Maximize under the pointer; a maximized window shows Restore"),h(R.Titlebar,{page:"Starfall 64",hover:"max",maximized:true})),
  h("div",null,h("div",{className:"ac-cap"},"Close under the pointer"),h(R.Titlebar,{page:"Settings",hover:"close"})))''')
readme("Titlebar", """
The window's title bar when Reclaw draws its own frame (the default on Linux, where GNOME leaves decoration to the application). Rust: `reclaw_ui::window::Titlebar`; 36px tall (`TITLEBAR_H`, a Rust constant), buttons 46 wide.

* The drag area is the app name and the page being shown; dragging moves the window, a double press maximizes or restores. Resize bands along the edges and the rounded corners come from the toolkit's `BorderlessPlugin`.
* Minimize, maximize or restore, and close. Hover is `bg-raised`; close goes `danger` with `on-accent` text, like every desktop's.
* Desktop interface only: Deck mode fills the screen and has no use for window buttons. `RECLAW_WINDOW_FRAME=native` turns the custom frame off.
* Every button emits `Effect::Window(WindowCommand)`; nothing here calls the windowing library.

Verified on X11 (Xvfb) with `scripts/x11-smoke.sh`; **not verified on Wayland/GNOME**. Transparent windows depend on the compositor.
""")

# ---- RemoteArt
card("Media", "RemoteArt", 280, "Artwork from the internet: a placeholder until the file is on disk, and if it never arrives",
 '''h("div",{className:"row"},
  h("div",null,h("div",{className:"ac-cap"},"Loading"),h(R.RemoteArt,{state:"loading"})),
  h("div",null,h("div",{className:"ac-cap"},"Ready"),h(R.RemoteArt,{state:"ready"})),
  h("div",null,h("div",{className:"ac-cap"},"Failed or off"),h(R.RemoteArt,{state:"failed"})))''')
readme("RemoteArt", """
A picture from the internet in the place of a placeholder. Rust: `reclaw_ui::components::RemoteArt`, built on `reclaw_ui::media::use_remote_file`.

* The placeholder (`ArtPlaceholder`, which names the art's role: CAPSULE, HEADER, SCREENSHOT ...) shows until the picture is on disk, and stays if it never arrives: offline, no art, or "Download artwork and READMEs" switched off in Settings.
* The picture fills the box and is cropped to it, like cover art. SVGs are drawn with the toolkit's SVG viewer and fall back to the placeholder if they cannot be drawn.
* It never fetches by itself: the file comes from `reclaw-media`'s cache (https only, nothing on the local network, size and time capped, kind decided from the bytes). A component using it is keyed by the address.

A failed picture is **not** reported to the user per image; the placeholder is the whole message. A cache size and a Clear cache button are not built yet.
""")

# ---- BannerArt
card("Media", "BannerArt", 280, "The game page's wide banner: a picture when there is one, a generated banner when there is not",
 '''h("div",{className:"row",style:{alignItems:"flex-start"}},
  [["Starfall 64","Generated"],["Tide Racer","Another game"],["Ship of Harkinian","Same name, same colour"]].map(function(g){return h("div",{key:g[0],style:{width:250}},h("div",{className:"ac-cap"},g[1]),h(R.BannerArt,{seed:g[0],height:180,strip:60}))}),
  h("div",{style:{width:250}},h("div",{className:"ac-cap"},"README or catalog"),h(R.BannerArt,{kind:"picture",tag:"README PICTURE",height:180})))''')
readme("BannerArt", """
The picture at the top of a game's page. Rust: `reclaw_ui::components::BannerArt`; the colours come from `reclaw_ui::banner`. The catalog mostly carries icons, so a page can rarely count on a wide picture. It uses the best one there is, in this order:

1. **The catalog's banner** (`reclaw.heroUrl`), trusted as it is.
2. **A picture from the project's README**: up to three, best first, ranked from the text alone (words like banner, header, screenshot; badges, buttons and sponsor images are never candidates). One is used only when its real size is at least 480 px wide and 1.6:1 or wider, so an icon is not blown up and a square is not cropped to a stripe.
3. **A generated banner**: the game's own colour (a hue taken from its title with FNV-1a, so it is the same on every run and every machine) mixed into the theme's background, with the game's icon blurred behind the icon itself, kept clear of the title strip. It needs no network, so the box is never empty and never a label.

Whatever fails to arrive (offline, a dead link, something that is not an image, "Download artwork and READMEs" switched off) falls through to the next step. Pictures are cropped to the box like cover art. In this mock the icon is the title's first letter, standing in for the catalog icon.

Don't: show a placeholder label such as HERO on a real page; stretch a small or square picture to fill the banner; put the title on the art without the solid strip (`HeroHeader` does this).
""")
