/* @ds-bundle: {"format":4,"namespace":"Reclaw","components":[{"name":"Button"},{"name":"Chip"},{"name":"StatusBadge"},{"name":"Switch"},{"name":"SearchField"},{"name":"GameCapsule"},{"name":"LibraryRow"},{"name":"HeroHeader"},{"name":"DownloadItem"},{"name":"Nav"},{"name":"InstallDialog"},{"name":"DeckTile"},{"name":"ButtonGlyph"},{"name":"HintBar"},{"name":"LaunchButton"},{"name":"NowPlayingBanner"},{"name":"SectionTabs"},{"name":"DeckPanel"}]} */
(function () {
  var R = window.React, h = R.createElement;
  var cx = function () { return Array.prototype.filter.call(arguments, Boolean).join(" "); };

  /* Stand-in glyphs for previews only. Freya uses freya_icons::lucide::<name>; the Lucide name is in each entry. */
  var ICONS = {
    play: "M7 4l13 8-13 8z", download: "M12 4v11m0 0l-4-4m4 4l4-4M5 20h14", check: "M5 12l5 5 9-10",
    alert: "M12 8v5m0 4h.01M10.3 4l-8 14a2 2 0 001.7 3h16a2 2 0 001.7-3l-8-14a2 2 0 00-3.4 0z",
    x: "M6 6l12 12M18 6L6 18", search: "M11 18a7 7 0 100-14 7 7 0 000 14zm9 3l-4.5-4.5",
    library: "M4 4h6v16H4zM14 4h6v7h-6zM14 15h6v5h-6z", catalog: "M4 7l8-4 8 4v10l-8 4-8-4zM4 7l8 4m0 0l8-4m-8 4v10",
    mods: "M10 4h4v3a2 2 0 104 0V4h2v6h-3a2 2 0 100 4h3v6h-6v-3a2 2 0 10-4 0v3H4v-6h3a2 2 0 100-4H4V4z",
    queue: "M12 4v12m0 0l-4-4m4 4l4-4M5 20h14", settings: "M12 15a3 3 0 100-6 3 3 0 000 6zM4 12h2m12 0h2M12 4v2m0 12v2",
    folder: "M3 6h6l2 2h10v11H3z", chevron: "M9 6l6 6-6 6", refresh: "M20 11a8 8 0 10-2.3 5.7M20 4v7h-7", file: "M6 3h8l4 4v14H6zM14 3v4h4", stop: "M6 6h12v12H6z", circle: "M12 4a8 8 0 100 16 8 8 0 000-16z"
  };
  var LUCIDE = { play: "play", download: "download", check: "check", alert: "triangle-alert", x: "x", search: "search", library: "library-big", catalog: "store", mods: "puzzle", queue: "arrow-down-to-line", settings: "settings", folder: "folder-open", chevron: "chevron-right", refresh: "refresh-cw", file: "file", stop: "square", circle: "circle" };
  function Icon(p) {
    var s = p.size || 16;
    return h("svg", { className: "rc-icon", width: s, height: s, viewBox: "0 0 24 24", fill: p.fill ? "currentColor" : "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true, "data-lucide": LUCIDE[p.name] }, h("path", { d: ICONS[p.name] || ICONS.file }));
  }

  function Button(p) {
    var v = p.variant || "secondary";
    return h("button", { className: cx("rc rc-btn", v, p.size, p.state && "is-" + p.state, p.disabled && "is-disabled"), disabled: p.disabled }, p.icon && h(Icon, { name: p.icon, size: p.size ? 18 : 16, fill: p.icon === "play" }), p.children);
  }
  function Chip(p) { return h("button", { className: cx("rc rc-chip", p.selected && "sel") }, p.children, p.count != null && h("span", { className: "n" }, p.count)); }

  var BADGES = {
    installed: ["ok", "check", "Installed"], update: ["warn", "download", "Update ready"], installing: ["info", "queue", "Installing"],
    failed: ["danger", "alert", "Failed"], available: ["none", null, "Not installed"], needsfile: ["warn", "file", "Needs your game file"]
  };
  function StatusBadge(p) {
    var b = BADGES[p.kind || "available"];
    return h("span", { className: cx("rc rc-badge", b[0]) }, b[1] && h(Icon, { name: b[1], size: 12 }), p.label || b[2]);
  }
  function Switch(p) { return h("span", { className: cx("rc rc-switch", p.on && "on", p.disabled && "dis"), role: "switch", "aria-checked": !!p.on }); }
  function SearchField(p) {
    return h("div", { className: cx("rc rc-search", p.size, p.focus && "focus") }, h(Icon, { name: "search", size: 16 }), p.value ? h("span", { className: "v" }, p.value) : h("span", { className: "ph" }, p.placeholder || "Search library, tags, repos"));
  }
  function Art(p) { return h("div", { className: cx("rc-art", p.className), style: p.style }, h("span", null, p.label || "ART")); }

  function GameCapsule(p) {
    return h("div", { className: cx("rc rc-capsule", p.selected && "sel", p.hover && "is-hover", p.fluid && "fluid") },
      h(Art, { label: "CAPSULE 3:4" }),
      p.status === "installed" && h("div", { className: "go" }, h(Button, { variant: "install", icon: "play" }, "Play")),
      h("div", { className: "b" }, h("div", { className: "t" }, p.title), h("div", { className: "rc-meta" }, p.project), h(StatusBadge, { kind: p.status })));
  }
  function LibraryRow(p) {
    var tone = { installed: "ok", update: "warn", installing: "info", failed: "danger", needsfile: "warn", available: "none" }[p.status || "available"];
    return h("div", { className: cx("rc rc-row", p.selected && "sel", p.hover && "is-hover", p.size) }, h("span", { className: "ic" }), h("span", { className: "nm" }, p.title), p.version && h("span", { className: "rc-mono", style: { color: "var(--ink-subtle)" } }, p.version), h("span", { className: "dot " + tone, title: p.status }));
  }
  function HeroHeader(p) {
    var inst = p.status === "installed" || p.status === "update";
    return h("div", { className: cx("rc rc-hero", p.narrow && "narrow") },
      h("div", { className: "rc-art art" }, h("span", null, "HERO 16:5"), h("div", { className: "ttl" }, h("div", { className: "rc-eyebrow", style: { color: "var(--accent)" } }, p.project), h("h1", null, p.title))),
      h("div", { className: "bar" },
        h(Button, { variant: "install", size: "lg", icon: p.status === "update" ? "download" : inst ? "play" : "download" }, p.status === "update" ? "Update" : inst ? "Play" : "Install"),
        h("div", { className: "kv" }, h("span", { className: "rc-eyebrow", style: { color: "var(--ink-subtle)" } }, "Version"), h("span", { className: "rc-mono" }, p.version)),
        h("div", { className: "kv" }, h("span", { className: "rc-eyebrow", style: { color: "var(--ink-subtle)" } }, "Source"), h("span", { className: "rc-mono" }, p.source || "github.com")),
        h(StatusBadge, { kind: p.status }),
        h("span", { style: { flex: 1 } }),
        h(Button, { variant: "ghost", icon: "folder", "aria-label": "Open folder" }), h(Button, { variant: "ghost", icon: "settings", "aria-label": "Manage" })));
  }
  function ProgressBar(p) { return h("div", { className: cx("rc rc-progress", p.tone), role: "progressbar", "aria-valuenow": p.value }, h("i", { style: { width: p.value + "%" } })); }
  function DownloadItem(p) {
    return h("div", { className: "rc rc-dl" }, h(Art, { className: "art", label: "HDR" }),
      h("div", { className: "m" }, h("div", { className: "h" }, h("span", { className: "nm" }, p.title), h("span", { className: "rc-mono", style: { color: "var(--ink-muted)" } }, p.stage)),
        h(ProgressBar, { value: p.value, tone: p.tone }), h("div", { className: "h rc-meta" }, h("span", null, p.detail), h("span", { className: "rc-mono" }, p.speed))),
      h(Button, { variant: "ghost", icon: "x" }));
  }
  var NAV = [["library", "library", "Library"], ["catalog", "catalog", "Catalog"], ["queue", "queue", "Downloads"], ["mods", "mods", "Mods"]];
  function Nav(p) {
    var mode = p.mode || "top";
    return h("nav", { className: cx("rc rc-nav", mode), style: p.style },
      mode === "top" && h("span", { style: { font: "700 15px/20px var(--font-sans)", color: "var(--ink)", marginRight: 8 } }, "Reclaw"),
      NAV.map(function (n) {
        return h("div", { key: n[0], className: cx("it", p.active === n[0] && "on", mode === "top" && "rc-eyebrow") }, h(Icon, { name: n[1], size: mode === "rail" ? 22 : mode === "bottom" ? 22 : 15 }), mode !== "rail" && h("span", null, n[2]));
      }),
      mode === "top" && h("span", { style: { flex: 1 } }), mode === "top" && h("div", { style: { width: 280 } }, h(SearchField, null)));
  }
  function InstallDialog(p) {
    return h("div", { className: "rc rc-scrim" }, h("div", { className: "rc-dialog" },
      h("div", { className: "hd" }, "Install " + (p.title || "Game")),
      h("div", { className: "bd" },
        h("div", { className: "rc-field" }, h("span", { className: "lb" }, "Install location"), h("div", { className: "rc-search" }, h(Icon, { name: "folder", size: 16 }), h("span", { className: "v rc-mono" }, "~/Reclaw/Apps/" + (p.slug || "game")))),
        h("div", { className: "rc-field" }, h("span", { className: "lb" }, "Your game file (never downloaded for you)"), h("div", { className: "rc-search" }, h(Icon, { name: "file", size: 16 }), h("span", { className: p.file ? "v rc-mono" : "ph" }, p.file || "Choose the file you own"))),
        h("div", { className: "rc-opt" }, h("span", null, "Create desktop shortcut"), h(Switch, { on: true })),
        h("div", { className: "rc-opt" }, h("span", null, "Keep pre-release builds"), h(Switch, { on: false }))),
      h("div", { className: "ft" }, h(Button, { variant: "ghost" }, "Cancel"), h(Button, { variant: "install", disabled: !p.file, icon: "download" }, "Install"))));
  }

  /* Showcase only (not in the bundle header): the three layout classes composed from the parts above. */
  var GAMES = [["Starfall 64", "N64Recomp", "installed", "v1.4.2"], ["Skyward Quest", "Zelda-style port", "update", "v0.9.1"], ["Kart Ruins", "N64Recomp", "needsfile", "v0.3.0"], ["Dino Rush", "PS2 recomp", "available", ""], ["Moon Garden", "GBA recomp", "failed", "v2.0.0"], ["Tide Racer", "N64Recomp", "installed", "v1.0.0"]];
  function LibraryScreen(p) {
    var mode = p.mode, sel = GAMES[0];
    var hero = h(HeroHeader, { title: sel[0], project: sel[1], version: sel[3], status: sel[2], narrow: mode !== "wide" });
    var rows = GAMES.map(function (g, i) { return h(LibraryRow, { key: i, title: g[0], status: g[2], selected: i === 0, size: mode === "phone" ? "touch" : null }); });
    var grid = function (cols) { return h("div", { className: "rc-grid", style: { gridTemplateColumns: "repeat(" + cols + ",1fr)" } }, GAMES.map(function (g, i) { return h(GameCapsule, { key: i, title: g[0], project: g[1], status: g[2], fluid: true, selected: i === 0 }); })); };
    if (mode === "wide") return h("div", { className: "rc rc-frame", style: { width: 1100, height: 640, flexDirection: "column" } },
      h(Nav, { mode: "top", active: "library" }),
      h("div", { style: { display: "flex", flex: 1, minHeight: 0 } },
        h("div", { className: "rc-side" }, h(SearchField, { placeholder: "Filter library" }), h("div", { style: { display: "flex", gap: 6, flexWrap: "wrap" } }, h(Chip, { selected: true, count: 6 }, "All"), h(Chip, { count: 2 }, "Installed"), h(Chip, { count: 1 }, "Updates")), h("div", { className: "rc-eyebrow", style: { color: "var(--ink-subtle)", marginTop: 8 } }, "N64 recompiled"), rows),
        h("div", { className: "col", style: { padding: 24, gap: 24, overflow: "hidden" } }, hero, h("div", { style: { display: "flex", gap: 16 } }, h("div", { style: { flex: 1 } }, h(DownloadItem, { title: "Starfall 64 1.4.3", stage: "Verifying", value: 62, detail: "38 MB of 61 MB", speed: "7.4 MB/s" }))))),
      h("div", { className: "rc-status" }, h("span", null, "Library synced"), h("span", { className: "rc-mono" }, "6 apps  2 updates")));
    if (mode === "compact") return h("div", { className: "rc rc-frame", style: { width: 860, height: 560 } },
      h(Nav, { mode: "rail", active: "library" }),
      h("div", { className: "col" }, h("div", { style: { padding: "12px 16px", borderBottom: "1px solid var(--line)" } }, h(SearchField, null)), h("div", { style: { padding: 16 } }, hero), grid(4)));
    return h("div", { className: "rc rc-frame", style: { width: 390, height: 780, flexDirection: "column" } },
      h("div", { style: { padding: "12px 16px", display: "flex", flexDirection: "column", gap: 12, borderBottom: "1px solid var(--line)" } }, h("div", { className: "rc-eyebrow", style: { color: "var(--ink-muted)" } }, "Library"), h(SearchField, { size: "touch" }), h("div", { style: { display: "flex", gap: 8 } }, h(Chip, { selected: true, count: 6 }, "All"), h(Chip, { count: 2 }, "Installed"), h(Chip, { count: 1 }, "Updates"))),
      h("div", { style: { flex: 1, overflow: "hidden" } }, grid(2)),
      h(Nav, { mode: "bottom", active: "library" }));
  }

  /* ---- Deck mode stand-ins. Rust: reclaw_ui::deck. Glyph faces mirror reclaw_input::ControllerKind::glyph. ---- */
  var GLYPHS = {
    xbox: { confirm: "A", back: "B", secondary: "X", prev: "LB", next: "RB", quick: "View", menu: "Guide" },
    playstation: { confirm: "x", back: "o", secondary: "sq", prev: "L1", next: "R1", quick: "Create", menu: "PS" },
    nintendo: { confirm: "B", back: "A", secondary: "Y", prev: "L", next: "R", quick: "-", menu: "Home" },
    steamdeck: { confirm: "A", back: "B", secondary: "X", prev: "L1", next: "R1", quick: "View", menu: "Steam" },
    keyboard: { confirm: "Enter", back: "Esc", secondary: "", prev: "[", next: "]", quick: "Shift+Tab", menu: "Tab" }
  };
  var PS = { x: "x", o: "circle", sq: "stop" };
  function ButtonGlyph(p) {
    var face = p.face, key = p.keyboard;
    var inner = !key && p.kind === "playstation" && PS[face] ? h(Icon, { name: PS[face] === "stop" ? "stop" : PS[face], size: 16 }) : face;
    return h("span", { className: cx("dk dk-glyph", key && "key") }, inner);
  }
  function HintBar(p) {
    var set = GLYPHS[p.kind || "xbox"], kb = p.kind === "keyboard";
    return h("div", { className: "dk dk-hints" }, (p.hints || [["confirm", "Select"], ["prev", "Previous"], ["next", "Next"], ["quick", "Quick access"], ["menu", "Menu"]]).map(function (x) {
      if (kb && !set[x[0]]) return null;
      return h("span", { key: x[0], className: "h" }, h(ButtonGlyph, { face: set[x[0]], kind: p.kind, keyboard: kb }), x[1]);
    }));
  }
  function DeckTile(p) {
    var g = p.game, badge = p.running ? h(StatusBadge, { kind: "installed", label: "Running" }) : h(StatusBadge, { kind: g.status, label: g.status === "needsfile" ? "Needs game file" : undefined });
    return h("div", { className: "dk dk-tile" }, h("div", { className: cx("art", "dk-art", p.focused && "dk-focus") }, h("span", null, "CAPSULE 3:4")),
      h("div", { className: "capt" }, h("div", { className: "t" }, g.title), (p.focused || p.running) && badge));
  }
  /* Same table as deck::launch::launch_verb. */
  var VERBS = {
    install: ["Install", "download", "install"], installing: ["Installing", "queue", "install", true], update: ["Update", "download", "install"],
    play: ["Play", "play", "install"], starting: ["Starting", "queue", "install", true], resume: ["Resume", "play", "install"],
    stop: ["Stop", "stop", "danger"], force: ["Force quit", "stop", "danger"], retry: ["Retry", "refresh", "install"]
  };
  function LaunchButton(p) {
    var v = VERBS[p.verb];
    return h("div", { className: "dk" }, h("div", { style: { display: "flex", gap: 16, alignItems: "center" } },
      h("span", { className: cx("dk-btn", v[2], v[3] && "off", p.focus && "focus") }, h(Icon, { name: v[1], size: 22 }), v[0]),
      p.pair && h("span", { className: "dk-btn danger" }, h(Icon, { name: "stop", size: 22 }), "Stop")),
      p.message && h("div", { className: "dk-msg" }, p.message));
  }
  function NowPlayingBanner(p) {
    return h("div", { className: "dk dk-banner" }, h(StatusBadge, { kind: "installed", label: "Running" }),
      h("div", { style: { flex: 1 } }, h("div", { style: { font: "600 18px/24px var(--font-sans)" } }, p.title), h("div", { style: { font: "400 16px/22px var(--font-sans)", color: "var(--ink-muted)" } }, "Running for 12 min")),
      h("span", { className: cx("dk-btn install", p.focus === "resume" && "focus") }, h(Icon, { name: "play", size: 22 }), "Resume"),
      h("span", { className: cx("dk-btn danger", p.focus === "stop" && "focus") }, h(Icon, { name: "stop", size: 22 }), "Stop"));
  }
  function SectionTabs(p) {
    var kind = p.kind || "xbox", set = GLYPHS[kind];
    return h("div", { className: "dk dk-tabs" }, h(ButtonGlyph, { face: set.prev, kind: kind }),
      ["Library", "Catalog", "Downloads", "Mods"].map(function (t) { return h("div", { key: t, className: cx("dk-tab", t === (p.current || "Library") && "on") }, t); }),
      h(ButtonGlyph, { face: set.next, kind: kind }));
  }
  /* A slide-in panel with its content: side "left" is the main menu, "right" is Quick access. */
  function DeckPanel(p) {
    if (p.side === "left") {
      var items = [["library", "Library"], ["catalog", "Catalog"], ["queue", "Downloads"], ["mods", "Mods"], ["settings", "Settings"], ["library", "Switch to desktop mode"]];
      return h("div", { className: "dk dk-panel left" }, h("div", { className: "dk-head" }, "Reclaw"),
        items.map(function (it, i) { return h("div", { key: i, className: cx("dk-row", i === (p.focus || 0) && "focus", i === 0 && "cur") }, h(Icon, { name: it[0], size: 24 }), it[1]); }));
    }
    return h("div", { className: "dk dk-panel right" }, h("div", { className: "dk-head" }, "Quick access"),
      p.running && h("div", null, h("div", { className: "dk-eyebrow" }, "Now playing"),
        h("div", { style: { padding: "0 24px" } }, h("div", { style: { font: "600 18px/24px var(--font-sans)" } }, "Starfall 64"), h("div", { style: { font: "400 16px/22px var(--font-sans)", color: "var(--ink-muted)", marginBottom: 12 } }, "Running for 12 min"),
          h("div", { style: { display: "flex", gap: 16 } }, h("span", { className: "dk-btn install focus" }, h(Icon, { name: "play", size: 22 }), "Resume"), h("span", { className: "dk-btn danger" }, h(Icon, { name: "stop", size: 22 }), "Stop")))),
      h("div", { className: "dk-eyebrow" }, "Controller"), h("div", { style: { padding: "0 24px", font: "400 20px/28px var(--font-sans)" } }, "Xbox Wireless Controller", h("div", { style: { font: "400 16px/22px var(--font-sans)", color: "var(--ink-muted)" } }, "Battery 82%")),
      h("div", { className: "dk-eyebrow" }, "Downloads"), h("div", { className: "dk-row" }, h(Icon, { name: "queue", size: 24 }), "Open the download queue"));
  }
  var DGAMES = [["Starfall 64", "installed"], ["Skyward Quest", "update"], ["Tide Racer", "installed"], ["Kart Ruins", "needsfile"], ["Dino Rush", "available"]];
  function DeckScreen(p) {
    var v = p.view || "home";
    var tiles = function (list, focusIdx, running) { return h("div", { style: { display: "flex", gap: 24, padding: "24px 24px 0" } }, list.map(function (g, i) { return h(DeckTile, { key: i, game: { title: g[0], status: g[1] }, focused: i === focusIdx, running: running && i === 0 }); })); };
    var page = v === "game"
      ? h("div", { style: { padding: "32px 0 0", width: 720 } }, h("div", { className: "rc-eyebrow", style: { color: "var(--accent)" } }, "N64Recomp"), h("div", { style: { font: "700 40px/44px var(--font-sans)", margin: "16px 0" } }, "Starfall 64"),
          h("div", { style: { display: "flex", gap: 16, alignItems: "center", marginBottom: 40 } }, h(StatusBadge, { kind: "installed", label: p.running ? "Running" : undefined }), h("span", { className: "rc-mono" }, "v1.4.2")),
          h(LaunchButton, { verb: p.running ? "resume" : "play", pair: p.running, focus: true }),
          h("div", { style: { display: "flex", gap: 16, marginTop: 24 } }, h("span", { className: "dk-btn secondary" }, h(Icon, { name: "folder", size: 22 }), "Open folder"), h("span", { className: "dk-btn secondary" }, h(Icon, { name: "settings", size: 22 }), "Manage")))
      : h("div", null, p.running && h("div", { style: { marginBottom: 24 } }, h(NowPlayingBanner, { title: "Starfall 64", focus: "resume" })),
          h("div", { className: "dk dk-shelf-title" }, "Continue"), tiles([DGAMES[0], DGAMES[1], DGAMES[2]], p.running ? -1 : 0, p.running),
          h("div", { className: "dk dk-shelf-title", style: { marginTop: 32 } }, "All apps"));
    return h("div", { className: "dk dk-room", style: { width: 1280, height: 800 } }, h("div", { className: "dk-wash" }),
      h("div", { style: { position: "absolute", inset: 0, padding: "32px 48px", display: "flex", flexDirection: "column" } },
        h(SectionTabs, null), h("div", { style: { flex: 1, paddingTop: 16, overflow: "hidden" } }, page),
        h(HintBar, { hints: v === "game" ? [["confirm", "Select"], ["back", "Library"], ["secondary", "Manage"], ["quick", "Quick access"]] : p.panel ? [["confirm", "Select"], ["back", "Close"]] : null })),
      p.panel && h("div", { className: "dk-scrim" }), p.panel && h(DeckPanel, { side: p.panel, running: p.running }));
  }

  window.Reclaw = { Button: Button, Chip: Chip, StatusBadge: StatusBadge, Switch: Switch, SearchField: SearchField, GameCapsule: GameCapsule, LibraryRow: LibraryRow, HeroHeader: HeroHeader, DownloadItem: DownloadItem, ProgressBar: ProgressBar, Nav: Nav, InstallDialog: InstallDialog, Icon: Icon, Art: Art, LibraryScreen: LibraryScreen, DeckTile: DeckTile, ButtonGlyph: ButtonGlyph, HintBar: HintBar, LaunchButton: LaunchButton, NowPlayingBanner: NowPlayingBanner, SectionTabs: SectionTabs, DeckPanel: DeckPanel, DeckScreen: DeckScreen };
})();
