# single source of truth for color tokens: name -> (midnight, daylight, usage)
def lum(h):
    h=h.lstrip('#'); r,g,b=[int(h[i:i+2],16)/255 for i in (0,2,4)]
    f=lambda c: c/12.92 if c<=0.03928 else ((c+0.055)/1.055)**2.4
    return 0.2126*f(r)+0.7152*f(g)+0.0722*f(b)
def cr(a,b):
    la,lb=sorted([lum(a),lum(b)],reverse=True); return (la+0.05)/(lb+0.05)

C=[
("bg-deep","#0b1017","#dde5ee","Window title bar and the bottom status strip. Darkest surface."),
("bg-nav","#101820","#e8eef4","Top navigation bar, icon rail and bottom tab bar."),
("bg-base","#16202b","#f2f5f9","App background behind everything; the library list well and page body."),
("bg-panel","#1c2937","#ffffff","Cards, capsules, dialogs, detail panels. Sits on bg-base."),
("bg-raised","#26374a","#e3eaf2","Inputs, hovered rows, selected row fill, art placeholders. Sits on bg-panel or bg-base."),
("line","#2c3e52","#cfd9e4","Decorative hairlines and dividers only. Never the sole boundary of a control."),
("line-strong","#5b7390","#6f8297","Borders of inputs, switches and unselected chips. 3:1 on bg-base and bg-panel."),
("ink","#e8eef5","#14202c","Primary text on bg-base, bg-panel, bg-raised, bg-nav and bg-deep."),
("ink-muted","#a9b8c9","#445568","Secondary text and inactive nav labels on bg-base, bg-panel, bg-raised, bg-nav. Also allowed over art on deck-scrim."),
("ink-subtle","#8496aa","#58697c","Placeholders, metadata (size, date, version) on bg-base, bg-panel, bg-nav, deck-bg. Not for disabled text, and never over art: on deck-scrim over a white image it reaches only 3.8:1."),
("accent","#66c0f4","#0b6aa8","Links, selected state, focus ring, progress fill for downloads, primary-button fill. Readable as text on every surface."),
("accent-hover","#8fd2f8","#0a5a90","Hover and pressed state of accent controls and links."),
("on-accent","#08141f","#ffffff","Text and icons on an accent or accent-hover fill."),
("install","#37790f","#2f7a12","Fill of the Install / Play / Update call to action, one per screen region. Steam-style green."),
("install-hover","#2f6a0b","#276a0f","Hover and pressed fill of the install button."),
("on-install","#ffffff","#ffffff","Text and icons on an install or install-hover fill."),
("ok","#86cc4a","#2d7a14","Installed / verified status text and icon on bg-panel, bg-base and ok-bg."),
("ok-bg","#1f3318","#e1f1d6","Fill behind an ok status badge."),
("warn","#f3bd52","#7a4f00","Update available / needs your file status text on bg-panel, bg-base and warn-bg."),
("warn-bg","#3a2f12","#fbefcf","Fill behind a warn status badge."),
("danger","#ff8484","#b3261e","Failed / error text and icon on bg-panel, bg-base and danger-bg."),
("danger-bg","#3d1c1f","#fbdcda","Fill behind a danger badge and error banners."),
("info","#66c0f4","#0b6aa8","Informational status text (queued, installing) on bg-panel, bg-base and info-bg."),
("info-bg","#12303f","#d8ecf8","Fill behind an info badge."),
("scrim","#05080cb3","#0b1017a6","Modal backdrop behind dialogs and drawers."),
("deck-bg","#0a0e14","#f2f5f9","Deck mode page background and the base under the Big Art backdrop. Follows the theme: near-black in midnight, the page grey of bg-base in daylight."),
("deck-scrim","#0a0e14d1","#f2f5f9d1","82% deck-bg laid over Big Art wherever text sits on art (hero title, tile caption). Text over it must still hold 4.5:1 against the worst image: pure white under the dark scrim, pure black under the light one."),
("focus-glow","#66c0f473","#0b6aa873","Outer glow of the focused element in Deck mode: accent at 45%. Decoration only; the 3px accent border carries the focus meaning."),
("deck-dim","#05080cd9","#0b1017c2","Dims the Deck page behind a centered menu, a confirmation or a notice's details. Dark in both themes, like scrim but deeper, so the light panel on top stands out in daylight."),
("deck-dim-ink","#e8eef5","#f2f5f9","Text drawn straight on deck-dim (a centered menu's title). Light in both themes because the dim is dark in both."),
]
SURF=["bg-deep","bg-nav","bg-base","bg-panel","bg-raised"]
def over(fg_hex8, bg_hex):
    h=fg_hex8.lstrip('#'); a=int(h[6:8],16)/255; r,g,b=[int(h[i:i+2],16) for i in (0,2,4)]
    bh=bg_hex.lstrip('#'); br,bg_,bb=[int(bh[i:i+2],16) for i in (0,2,4)]
    return "#%02x%02x%02x"%(round(r*a+br*(1-a)),round(g*a+bg_*(1-a)),round(b*a+bb*(1-a)))
def checks():
    out=[]; d={n:(a,b) for n,a,b,_ in C if len(a)==7}
    for ti,theme in enumerate(["midnight","daylight"]):
        g=lambda n:d[n][ti]
        def chk(fg,bgs,mn,label=""):
            for bg in bgs: out.append((theme,fg,bg,mn,cr(g(fg),g(bg))))
        chk("ink",SURF,4.5); chk("ink-muted",["bg-nav","bg-base","bg-panel","bg-raised"],4.5)
        chk("ink-subtle",["bg-nav","bg-base","bg-panel"],4.5)
        chk("accent",["bg-nav","bg-base","bg-panel","bg-raised"],4.5); chk("accent-hover",["bg-base","bg-panel"],4.5)
        chk("on-accent",["accent","accent-hover"],4.5); chk("on-install",["install","install-hover"],4.5)
        chk("line-strong",["bg-base","bg-panel"],3.0)
        # deck: text on deck-bg, and on deck-scrim over the worst image: white under a dark scrim, black under a light one
        scrim=dict((n,(a,b)) for n,a,b,_ in C)["deck-scrim"][ti]
        for fg in ("ink","ink-muted","ink-subtle","accent"):
            out.append((theme,fg,"deck-bg",4.5,cr(g(fg),g("deck-bg"))))
        # Over art, only ink and ink-muted are allowed (ink-subtle misses on a white image).
        for fg in ("ink","ink-muted"):
            for image in ("#ffffff","#000000"):
                out.append((theme,fg,"deck-scrim over "+image,4.5,cr(g(fg),over(scrim,image))))
        # text straight on the dim, over the page it dims at its lightest (white) and darkest (black)
        dim=dict((n,(a,b)) for n,a,b,_ in C)["deck-dim"][ti]
        for image in ("#ffffff","#000000"):
            out.append((theme,"deck-dim-ink","deck-dim over "+image,4.5,cr(g("deck-dim-ink"),over(dim,image))))
        out.append((theme,"on-install","install",4.5,cr(g("on-install"),g("install"))))
        for s,bg in [("ok","ok-bg"),("warn","warn-bg"),("danger","danger-bg"),("info","info-bg")]:
            chk(s,[bg,"bg-panel","bg-base"],4.5)
    return out
if __name__=="__main__":
    bad=0
    for t,f,b,mn,v in checks():
        if v<mn: bad+=1; print("FAIL",t,f,"on",b,round(v,2),"need",mn)
    print("fails:",bad,"of",len(checks()))
