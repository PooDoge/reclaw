import os
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))  # work from design-system/, wherever this is run
import json, os
from colors import C
P="."; os.makedirs(P+"/freya",exist_ok=True)
col=[]
for n,a,b,u in C: col.append({"name":n,"value":{"midnight":a,"daylight":b},"usage":u})
sp=[("space-1","4px","Icon-to-label gap, badge padding."),("space-2","8px","Gap between related controls; list row vertical padding."),
("space-3","12px","Card inner padding on handheld; gap between capsules."),("space-4","16px","Default panel padding and page gutter on phone."),
("space-5","24px","Page gutter on desktop; gap between sections."),("space-6","32px","Hero inner padding on desktop."),("space-8","48px","Large section separation; touch row height.")]
rad=[("radius-sm","2px","Badges, progress bars, chips. Steam-like near-square corners."),("radius-md","4px","Buttons, inputs, capsules, list rows."),
("radius-lg","8px","Dialogs, drawers, bottom sheet top corners."),("radius-pill","999px","Switch track and knob only.")]
shadow=[("shadow-card","0 2px 8px rgba(0,0,0,0.35)","Capsule on hover; card resting on bg-base."),
("shadow-pop","0 12px 32px rgba(0,0,0,0.5)","Dialogs, menus, drawers."),
("shadow-focus","0 0 0 2px #66c0f4","Keyboard / gamepad focus ring. Solid accent, 3:1 on all surfaces. Freya: a 2px accent border, there is no spread shadow.")]
layout=[("sidebar-w","240px","Library list column at wide breakpoint."),("rail-w","64px","Icon rail at compact breakpoint."),
("topbar-h","40px","Top nav height on pointer devices."),("tabbar-h","56px","Bottom tab bar height on phone."),
("row-h","32px","Library row height, pointer density."),("row-h-touch","48px","Library row height, touch / controller density."),
("target-min","44px","Minimum hit target on touch density."),("capsule-w","168px","Game capsule width (3:4 portrait art)."),
("capsule-h","224px","Game capsule art height."),("hero-h","280px","Game page banner height at wide."),
("deck-safe-x","48px","Deck mode: horizontal safe-zone margin from the screen edge, for TV overscan. User-adjustable up to 96px."),
("deck-safe-y","32px","Deck mode: vertical safe-zone margin from the screen edge."),
("deck-tile-w","220px","Deck mode: game tile width (3:4 portrait art)."),
("deck-tile-h","293px","Deck mode: game tile height."),
("deck-tile-gap","24px","Deck mode: gap between tiles in a shelf; also the room the focus scale grows into."),
("deck-row-h","64px","Deck mode: list row height (menus, quick access). Twice the pointer row."),
("deck-target-min","56px","Deck mode: minimum hit and focus target."),
("deck-hint-h","56px","Deck mode: height of the button-hint bar along the bottom safe zone."),
("deck-tabs-h","64px","Deck mode: height of the section tab strip."),
("deck-panel-w","420px","Deck mode: width of the left and right slide-in panels."),
("deck-focus-scale","1.06","Deck mode: how much the focused element grows. Needs deck-tile-gap of room."),
("deck-focus-ring","3px","Deck mode: focus border width, solid accent."),
("deck-menu-w","416px","Deck mode: width of one column of a centered option menu; a submenu opens as a second column."),
("deck-settings-nav-w","400px","Deck mode: width of the section list beside the rows of a two-pane settings page."),
("deck-settings-row-h","68px","Deck mode: height of a settings row. Rows are one line, so descriptions are short. A text row adds 72px."),
("deck-two-pane-min-w","900px","Deck mode: windows at least this wide show settings as two panes; narrower ones drill from a list into a section."),
("surface-header-h","64px","Full-screen pages: header with Back and the title."),
("surface-header-h-compact","48px","Full-screen pages: header while a keyboard covers over 40% of the window."),
("surface-footer-h","88px","Full-screen pages: footer holding the actions."),
("surface-max-w","720px","Full-screen pages: widest the reading column grows; two-pane pages use the full width."),
("surface-short-h","600px","Full-screen pages: under this window height the footer actions move into the header."),
("surface-touch-popup-min-h","900px","A touch screen at least this tall keeps popup forms; shorter ones get full-screen pages."),
("bp-wide","1100px","Min width for full sidebar layout."),("bp-compact","720px","Min width for icon rail layout; below it is handheld / phone.")]
def fam(rows): return {"tokens":[{"name":n,"value":v,"usage":u} for n,v,u in rows]}
tok={"name":"Reclaw","version":1,
"color":{"themes":[{"id":"midnight","name":"Midnight"},{"id":"daylight","name":"Daylight"}],"tokens":col},
"type":{"fonts":[],"families":{"sans":"\"Hanken Grotesk\", system-ui, sans-serif","mono":"\"JetBrains Mono\", ui-monospace, monospace"},
"groups":[
{"name":"Display","family":"sans","styles":[
 {"name":"title-hero","fontSize":"32px","lineHeight":"36px","fontWeight":700,"usage":"Game name on the hero banner (wide)."},
 {"name":"title-page","fontSize":"22px","lineHeight":"28px","fontWeight":700,"usage":"Page title; hero name on phone."}]},
{"name":"Text","family":"sans","styles":[
 {"name":"heading","fontSize":"16px","lineHeight":"22px","fontWeight":600,"usage":"Panel and dialog titles."},
 {"name":"body","fontSize":"14px","lineHeight":"20px","fontWeight":400,"usage":"Default text, list rows, descriptions."},
 {"name":"body-touch","fontSize":"16px","lineHeight":"24px","fontWeight":400,"usage":"Body text at touch / controller density."},
 {"name":"label","fontSize":"13px","lineHeight":"16px","fontWeight":600,"usage":"Buttons, chips, tabs."},
 {"name":"meta","fontSize":"12px","lineHeight":"16px","fontWeight":400,"usage":"Secondary metadata: size, date, repo."},
 {"name":"eyebrow","fontSize":"11px","lineHeight":"14px","fontWeight":700,"letterSpacing":"0.08em","usage":"Uppercase section labels and top-nav items (set the string uppercase in code)."}]},
{"name":"Deck","family":"sans","styles":[
 {"name":"deck-title","fontSize":"40px","lineHeight":"44px","fontWeight":700,"usage":"Deck mode: game name on the hero."},
 {"name":"deck-heading","fontSize":"24px","lineHeight":"32px","fontWeight":600,"usage":"Deck mode: shelf titles, panel titles, section tabs."},
 {"name":"deck-body","fontSize":"20px","lineHeight":"28px","fontWeight":400,"usage":"Deck mode: body text and list rows."},
 {"name":"deck-label","fontSize":"18px","lineHeight":"24px","fontWeight":600,"usage":"Deck mode: buttons, tile titles."},
 {"name":"deck-meta","fontSize":"16px","lineHeight":"22px","fontWeight":400,"usage":"Deck mode: metadata. Not over art (use deck-label in ink)."},
 {"name":"deck-hint","fontSize":"16px","lineHeight":"20px","fontWeight":600,"usage":"Deck mode: button hint labels along the bottom bar."}]},
{"name":"Mono","family":"mono","styles":[
 {"name":"mono","fontSize":"12px","lineHeight":"16px","fontWeight":400,"usage":"Versions, tags like v1.4.2, hashes, install paths, release asset names."}]}]},
"spacing":fam(sp),"radius":fam(rad),"shadow":fam(shadow),"layout":fam(layout)}
json.dump(tok,open(P+"/tokens.json","w"),indent=1)

# Rust theme generated from the same tokens
def rgb(h):
    h=h.lstrip('#'); return tuple(int(h[i:i+2],16) for i in (0,2,4))
def cs(h): 
    if len(h)==9:
        r,g,b=rgb(h[:7]); a=int(h[7:9],16); return f"Color::from_argb({a}, {r}, {g}, {b})"
    r,g,b=rgb(h); return f"Color::from_rgb({r}, {g}, {b})"
def const(prefix, idx):
    L=[]
    for n,a,b,_ in C:
        L.append(f"    pub {n.replace('-','_')}: Color,")
    return "\n".join(L)
fields="\n".join(f"    pub {n.replace('-','_')}: Color," for n,*_ in C)
def init(idx):
    return "\n".join(f"        {n.replace('-','_')}: {cs((a,b)[idx])}," for n,a,b,_ in C)
get_arms="\n".join(f'            "{n}" => self.{n.replace("-","_")},' for n,*_ in C)
rs=f"""//! GENERATED from design-system/tokens.json. Edit the tokens and regenerate, not this file.
use freya::prelude::*;

/// Every Reclaw token, including the ones ColorsSheet has no slot for (install green,
/// status fills, bg_deep). Provide it with use_provide_context next to use_init_theme.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Reclaw {{
{fields}
}}

pub fn midnight() -> Reclaw {{
    Reclaw {{
{init(0)}
    }}
}}

pub fn daylight() -> Reclaw {{
    Reclaw {{
{init(1)}
    }}
}}

pub(crate) fn sheet(t: &Reclaw, base: ColorsSheet) -> ColorsSheet {{
    ColorsSheet {{
        primary: t.accent,
        secondary: t.install,
        tertiary: t.accent_hover,
        success: t.ok,
        warning: t.warn,
        error: t.danger,
        info: t.info,
        background: t.bg_base,
        surface_primary: t.bg_panel,
        surface_secondary: t.bg_raised,
        surface_tertiary: t.bg_nav,
        border: t.line_strong,
        border_focus: t.accent,
        text_primary: t.ink,
        text_secondary: t.ink_muted,
        text_placeholder: t.ink_subtle,
        text_inverse: t.on_accent,
        overlay: t.scrim,
        ..base
    }}
}}

impl Reclaw {{
    /// Look a token up by its design-system name (`"bg-panel"`). Used by the drift test.
    pub fn get(&self, name: &str) -> Option<Color> {{
        Some(match name {{
{get_arms}
            _ => return None,
        }})
    }}
}}

pub fn reclaw_midnight() -> Theme {{
    let mut theme = dark_theme();
    theme.name = "reclaw-midnight";
    theme.colors = sheet(&midnight(), DARK_COLORS);
    theme
}}

pub fn reclaw_daylight() -> Theme {{
    let mut theme = light_theme();
    theme.name = "reclaw-daylight";
    theme.colors = sheet(&daylight(), LIGHT_COLORS);
    theme
}}
"""
open(P+"/freya/theme.rs","w").write(rs)
open("../reclaw-ui/src/tokens.rs","w").write(rs)
# Keep the generated Rust in the workspace's style so `cargo fmt --check` stays clean.
import subprocess
for path in (P+"/freya/theme.rs", "../reclaw-ui/src/tokens.rs"):
    subprocess.run(["rustfmt", "--edition", "2024", "--config-path", "..", path], check=False)
