"""Adds the search button, a search's results line and empty state, and the Mods tab's shelves to the design system: patches
components/bundle.{js,css} between marker comments, points the Nav preview's top bar at the search button, and writes the
README + preview files. Safe to run again. Run from anywhere: `python3 design-system/tools/gen_search.py`."""
import os, re, json
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
exec(open("tools/gen_activity.py").read().split("patch(\"components/bundle.css\"")[0].replace('NAMES = [', 'ACTIVITY_NAMES = ['))

NAMES = ["SearchToggle", "ResultsBar", "NoResults", "ModShelf", "Highlight"]

patch("components/bundle.css", "/* search:start */", "/* search:end */", open("tools/search.css").read())
patch("components/bundle.js", "/* search:start */", "/* search:end */", open("tools/search.js").read())
js = open("components/bundle.js").read()
# The top bar's search is the button now, not a 280px field.
js = js.replace('mode === "top" && h("div", { style: { width: 280 } }, h(SearchField, null))', 'mode === "top" && h(window.Reclaw.SearchToggle, null)')
first, rest = js.split("\n", 1)
m = re.match(r"(/\* @ds-bundle: )(\{.*\})( \*/)", first)
manifest = json.loads(m.group(2))
manifest["components"] = [c for c in manifest["components"] if c["name"] not in NAMES]
manifest["components"] += [{"name": n} for n in NAMES]
open("components/bundle.js", "w").write(m.group(1) + json.dumps(manifest, separators=(",", ":")) + m.group(3) + "\n" + rest)

def readme(name, text):
    os.makedirs(f"{P}/{name}", exist_ok=True)
    open(f"{P}/{name}/README.md", "w").write(text.strip() + "\n")

MODS = [
    {"title": "Free Camera", "meta": "lookaround · Thunderstore · v1.0.3 · 12.9k downloads · 32 ratings · updated 3 days ago", "summary": "Adds a free camera with a photo mode."},
    {"title": "HD Texture Pack", "meta": "pixelwright · Thunderstore · v2.1.0 · 48.2k downloads · 120 ratings", "summary": "Replaces the hub world textures with 4x versions.", "installed": True},
    {"title": "Randomizer", "meta": "shuffler · GameBanana · v0.8.0 · 30.6k downloads · 76 likes", "summary": "Shuffles item locations for a new run every time."},
    {"title": "Hard Mode", "meta": "ironman · GameBanana · v1.2.0 · 8.1k downloads · 20 likes", "summary": "Enemies hit harder and hearts are rarer."},
]
M = json.dumps(MODS)

preview("SearchToggle", "Inputs", 170, "Press it: it opens to the left over what is beside it, and closes on Enter or a click away",
 '''h("div",{className:"col"},
  h("div",null,h("div",{className:"cap"},"Top bar (wide): closed, between Deck mode and the edge"),h("div",{className:"row",style:{justifyContent:"flex-end",gap:16,padding:"4px 16px",background:"var(--bg-nav)",maxWidth:620}},h(R.Button,{variant:"ghost",icon:"refresh"},"Recent"),h(R.Button,{variant:"ghost"},"Deck mode"),h(R.SearchToggle,{expanded:320}))),
  h("div",null,h("div",{className:"cap"},"Open, typing; and closed with a search running (the dot)"),h("div",{className:"row",style:{justifyContent:"flex-end",gap:16,padding:"4px 16px",background:"var(--bg-nav)",maxWidth:620}},h(R.SearchToggle,{open:true,value:"star",expanded:320}),h("span",{style:{width:24}}),h(R.SearchToggle,{active:true}))))''')
readme("SearchToggle", """
The search box of the desktop interface. Rust: `reclaw_ui::components::SearchToggle` (drawn), `reclaw_ui::desktop::DesktopSearch` (what it searches, and when it opens and closes), `reclaw_ui::search` (pure: `SearchModel`, `SearchScope`, matching, highlighting).

* **Closed:** a 32px square magnifier (`bg-panel`, `line` border, `radius-md`) at the right end of the top bar on a wide window. On compact and phone windows it floats at the top right of a tab's own page (36px with a pointer, 44px on touch), over the page, so it takes no row of its own.
* **Opening** (press, or Tab onto it): the bar slides open **to the left** over whatever is beside it (the Recent and Deck mode buttons; the page title on a phone), 240 ms expo-out, instant with reduced motion. Its own box never changes size: the bar is absolutely placed, anchored at its right edge, so nothing in the layout moves. Open: `bg-raised`, 2px `accent` border, `shadow-pop`; 320px wide in the top bar, the page's width less its gutters on a narrow window.
* **Searches the tab that is showing:** Library, Catalog, Mods (the chosen game's mods; the placeholder says which game), Settings. Downloads has nothing to search, and the button is not drawn there (its space stays). Each tab keeps its own search.
* **Closes** when the search is sent (Enter, or the magnifier while open) and when focus goes (a click elsewhere, Escape, another tab). What was typed but not sent is kept, and the box opens on it again; after two minutes unsent it is dropped. Sending from a page under a tab (a game, a mod) shows the results on the tab's page.
* **The dot:** an `accent` dot on the closed button while a search is narrowing the tab, so a short list is never a mystery.

Don't: search as each letter is typed (results follow what was sent, so a click on a result never lands on a list that just changed); push the bar's neighbours aside; show the box on a game's page on a narrow window (its own buttons are in that corner).
""")

preview("ResultsBar", "Inputs", 230, "Above a search's results; the narrow form in the Library sidebar; a search that found nothing",
 '''h("div",{className:"col",style:{maxWidth:640}},h(R.ResultsBar,{found:3,query:"camera",place:"in mods"}),h("div",{style:{width:260}},h(R.ResultsBar,{found:1,query:"tide",narrow:true})),h(R.NoResults,{query:"zzzz",place:"in the catalog"}))''')
readme("ResultsBar", """
The line above a search's results. Rust: `results_bar`, `no_results` and `highlighted` in `reclaw_ui::desktop::pages::common`.

* `bg-panel`, a 3px `accent` bar on the left, the magnifier in `accent`; "3 results for “camera”" in `label` `ink`, where it looked ("in mods") in `meta` `ink-subtle`, and **Clear search** (ghost, x). In the Library's sidebar (`narrow`) the place goes and Clear is its icon.
* **Nothing found** replaces the list: the magnifier, "Nothing in the catalog matches “zzzz”", a hint naming what else narrows the list (platform, site, filter), and Clear search (secondary). An empty list is explained, never blank.
* **Highlight:** in mod results the words that matched are `accent` and bold in the title (`search::highlight`, case-insensitive, in the title's own letters).
* **Settings** results are the real rows, working, under their section's title in an `accent` eyebrow: change a setting without going to its section (`search::filter_schema`; a row is found by its label, description or choices, or by its section's or group's title).
""")

preview("ModShelf", "Mods", 560, "The Mods tab's shelves: a few each and Show all; an opened shelf with its pager",
 f'''h("div",{{className:"col",style:{{gap:32}}}},
  h(R.ModShelf,{{kind:"popular",title:"Most downloaded",note:"All-time downloads",count:57,mods:{M}}}),
  h(R.ModShelf,{{kind:"updated",title:"Recently updated",note:"Newest version first",count:40,opened:true,pages:2,page:0,range:"1\\u201320 of 40",columns:1,mods:{json.dumps(MODS[:2])}}}))''')
readme("ModShelf", """
The Mods tab without a search. Rust: `reclaw_ui::desktop::pages::mods` (drawn), `reclaw_ui::mod_shelves` (pure: which mods, in which order, `Paging`).

* **Shelves, in order:** Installed (updates first), Most downloaded, Top rated (Thunderstore ratings, GameBanana likes), Recently updated, New releases. A shelf with nothing on it is left out. They follow the game chip and the site chip.
* **What is on a shelf is the site's own order.** The host asks each site for its mods in four orders (most downloaded 60, the others 20 each) and records each mod's place (`ModRanks`); a shelf shows the mods its order listed. Two sites on one shelf merge by the number the order is about (downloads, ratings, dates) where both give it, and by their places otherwise.
* **Header:** a 32px `bg-raised` tile with the shelf's icon in `accent` (check, download, star, refresh-cw, clock), the title in `heading`, the count in `mono` `ink-subtle`, what the order is in `meta`. **Show all N** (ghost, chevron-right) when there are more than shown.
* **Preview:** four mods in two columns on a wide window, three in one column on compact and phone.
* **Opened:** the whole shelf, 20 a page, **All shelves** (ghost, chevron-left) in the header's place of Show all, and a pager (Previous, "Page 1 of 3 · 1–20 of 57", Next) when it needs one. Search results page the same way.
* **Row:** title, then `meta`: author · site · version · downloads · ratings or likes · "updated 3 days ago", then the summary, and Install / Update / Remove.
""")
