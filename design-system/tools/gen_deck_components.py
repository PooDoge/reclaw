import os
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))  # work from design-system/, wherever this is run
exec(open("tools/gen_components.py").read().split('preview("Button"')[0])

def deck_preview(name, height, subtitle, body, group="Deck"):
    preview(name, group, height, subtitle, body, pad=24, extra_css="body{background:var(--deck-bg)}")
    p=f"{P}/{name}/preview.html"; s=open(p).read()
    # Deck follows the theme (ADR 0007): the preview shows whichever theme the host renders.
    open(p,"w").write(s)

G="{title:'Starfall 64',status:'installed'}"
deck_preview("DeckTile",470,"Rest, focused and running; the title always shows, the badge only on focus",
 f'''h("div",{{style:{{display:"flex",gap:32,padding:"12px 12px 0"}}}},h(R.DeckTile,{{game:{G}}}),h(R.DeckTile,{{game:{G},focused:true}}),h(R.DeckTile,{{game:{G},running:true}}),h(R.DeckTile,{{game:{{title:"Kart Ruins",status:"available"}},focused:true}}))''')
readme("DeckTile","""
Big-art game tile for Deck mode. Rust: `reclaw_ui::deck::DeckTile`.

The title is always visible (placeholder art has none baked in). Focus scales the tile 1.06, draws a 3px solid accent ring and a `focus-glow` halo; the badge appears on focus or while the app runs, in space reserved so focusing never reflows the shelf. Running takes priority over install state.

Consumer provides the game and whether it is focused (from `DeckState`, never from hover). The ring is drawn only after gamepad or keyboard input.
""")
deck_preview("ButtonGlyph",220,"Glyph faces per controller; neutral circle, ring and ink letter",
 '''h("div",{style:{display:"grid",gridTemplateColumns:"140px 1fr",gap:"14px 16px",alignItems:"center",color:"var(--ink-muted)",font:"600 16px var(--font-sans)"}},[["xbox","Xbox"],["playstation","PlayStation"],["nintendo","Nintendo"],["steamdeck","Steam Deck"],["keyboard","Keyboard"]].map(function(k){var g={xbox:["A","B","X","Y"],playstation:["x","o","sq"],nintendo:["B","A","Y","X"],steamdeck:["A","B","X","Y"],keyboard:["Enter","Esc","Tab"]}[k[0]];return [h("span",{key:k[0]+"l"},k[1]),h("span",{key:k[0],style:{display:"flex",gap:10}},g.map(function(f){return h(R.ButtonGlyph,{key:f,face:f,kind:k[0],keyboard:k[0]==="keyboard"})}))]}))''')
readme("ButtonGlyph","""
One controller button or keycap. Rust: `reclaw_ui::deck::ButtonGlyph`, faces from `reclaw_input::ControllerKind::glyph`.

Neutral on purpose: a `bg-raised` circle, `line-strong` 2px ring, `ink` letter, so contrast is identical for every controller and no vendor colors or marks are used. PlayStation face buttons are Lucide x, circle, square, triangle. Labels follow the cap (a Nintendo pad's East button reads A), while actions follow position (Confirm is South, which is B on a Nintendo pad unless the layout is swapped).
""")
deck_preview("HintBar",300,"Contextual prompts; glyph set follows the last-used device",
 '''h("div",{style:{display:"flex",flexDirection:"column",gap:8}},["xbox","playstation","nintendo","steamdeck","keyboard"].map(function(k){return h(R.HintBar,{key:k,kind:k})}))''')
readme("HintBar","""
Button prompts along the bottom safe zone. Rust: `reclaw_ui::deck::HintBar`.

Hints are contextual (Home: Select, Previous, Next, Quick access, Menu; Game page: Select, Library, Manage, Quick access; overlays: Select, Close). Glyphs come from `ActionMap::glyph(action, kind)`, so a rebinding changes the bar. After keyboard or mouse use the bar shows keycaps instead, and hints with no default key are hidden. Under Steam's virtual Xbox pad the glyphs read Xbox whatever hardware is in hand.
""")
deck_preview("LaunchButton",560,"Play turns into Stop (desktop) or Resume + Stop (deck); every state in one table",
 '''h("div",{style:{display:"grid",gridTemplateColumns:"260px 1fr",gap:"18px 24px",alignItems:"center",color:"var(--ink-muted)",font:"400 16px var(--font-sans)"}},[
["Installed, idle","play"],["Update ready","update"],["Not installed","install"],["Installing","installing"],["Starting","starting"],["Running (deck)","resume",true],["Running (desktop)","stop"],["Stopping: press again to force","force"],["Last run failed","retry",false,"Exited with code 3. The log has details."]
].map(function(r,i){return [h("span",{key:"l"+i},r[0]),h(R.LaunchButton,{key:"b"+i,verb:r[1],pair:r[2],message:r[3]})]}))''')
readme("LaunchButton","""
The primary verb for a game; the lifecycle in one control. Rust: `reclaw_ui::deck::{launch_verb, LaunchButton}`; the table is `deck/launch.rs` and `lifecycle.ui.LaunchButton` in the contract.

Desktop swaps Play for Stop while the app runs. Deck shows Resume (A) with Stop beside it, because Resume is almost always what you want and Stop must not be one mis-press away. Stop sends a graceful quit to the whole process group; **pressing it again while Stopping force-kills** (the button then reads Force quit). A failed run shows Retry and a plain-words reason, never a bare red state.
""")
deck_preview("NowPlayingBanner",180,"Top of Deck Home while an app is active; Resume focused",
 '''h("div",{style:{display:"flex",flexDirection:"column",gap:24}},h(R.NowPlayingBanner,{title:"Starfall 64",focus:"resume"}),h(R.NowPlayingBanner,{title:"Skyward Quest",focus:"stop"}))''')
readme("NowPlayingBanner","""
Running-app strip at the top of Deck Home. Rust: `reclaw_ui::deck::NowPlayingBanner`. Title, elapsed whole minutes, and Resume and Stop as two focus targets (default focus is Resume). While Stopping, Stop reads Force quit.
""")
deck_preview("SectionTabs",140,"Library, Catalog, Downloads, Mods with bumper glyphs",
 '''h("div",{style:{display:"flex",flexDirection:"column",gap:12}},h(R.SectionTabs,{kind:"xbox"}),h(R.SectionTabs,{kind:"playstation",current:"Downloads"}))''')
readme("SectionTabs","""
Section strip with the bumper glyphs at each end. Rust: `reclaw_ui::deck::SectionTabs`. Not a focus target: PrevSection and NextSection move it and wrap around. Selected tab is `ink` with a 4px accent underline; others `ink-muted`.
""")
deck_preview("DeckPanel",520,"Main menu (left) and Quick access (right), as they sit over the page",
 '''h("div",{style:{display:"flex",gap:24}},h("div",{style:{position:"relative",width:420,height:470}},h(R.DeckPanel,{side:"left",focus:1})),h("div",{style:{position:"relative",width:420,height:470}},h(R.DeckPanel,{side:"right",running:true})))''')
readme("DeckPanel","""
The two slide-in panels. Rust: `reclaw_ui::deck::{SlidePanel, MainMenu, QuickAccess}`.

They overlay the page and never shift it: an absolute root on `Layer::Overlay`, the `scrim` token over the rest of the window, 280ms expo-out slide. They trap focus; Back closes. Opening one over a running app (Guide) brings the launcher forward and takes the pad; closing it returns the pad to the app. Main menu rows are `deck-row-h` tall with an accent bar on the focused row (no scale, so rows stay aligned).
""")
# page: whole screens
os.makedirs(f"{P}/DeckScreens",exist_ok=True)
preview("DeckScreens","Deck",1900,"Home, game page, running app and Quick access at 1280x800",
 '''h("div",{style:{display:"flex",flexDirection:"column",gap:32,alignItems:"flex-start"}},[["home","Home","Library shelves with the first tile focused"],["home-run","Home, app running","Now Playing banner above the shelves",true],["game","Game page","Launch verb, secondary actions"],["quick","Quick access over a running app","Resume focused; the page dims under the scrim",true,"right"]].map(function(s,i){var z=Math.min(1,(window.innerWidth-48)/1280);return h("div",{key:i,style:{zoom:z}},h("p",{className:"cap"},s[1]+": "+s[2]),h(R.DeckScreen,{view:s[0]==="game"?"game":"home",running:!!s[3],panel:s[4]}))}))''',pad=24,extra_css="body{background:var(--deck-bg)}")
s=open(f"{P}/DeckScreens/preview.html").read().replace('subtitle="Home, game page, running app and Quick access at 1280x800"','page subtitle="Home, game page, running app and Quick access at 1280x800"').replace('<div id="r"></div>','<div id="r" data-theme="midnight"></div>')
open(f"{P}/DeckScreens/preview.html","w").write(s)
readme("DeckScreens","Reference compositions of Deck mode at 1280x800 (a Steam Deck's resolution). On a TV the same layout scales with the user's UI scale (1.0, 1.25, 1.5) inside the 48x32 safe zone. Real renders from the Rust crate are in `reclaw-ui/target/snapshots/deck-*.png` after `cargo test`.")
