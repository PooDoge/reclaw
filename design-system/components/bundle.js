/* @ds-bundle: {"format":4,"namespace":"Reclaw","components":[{"name":"Button"},{"name":"Chip"},{"name":"StatusBadge"},{"name":"Switch"},{"name":"SearchField"},{"name":"GameCapsule"},{"name":"LibraryRow"},{"name":"HeroHeader"},{"name":"DownloadItem"},{"name":"Nav"},{"name":"InstallDialog"},{"name":"DeckTile"},{"name":"ButtonGlyph"},{"name":"HintBar"},{"name":"LaunchButton"},{"name":"NowPlayingBanner"},{"name":"SectionTabs"},{"name":"DeckPanel"},{"name":"SettingRow"},{"name":"FullScreenPage"},{"name":"ModalMenu"},{"name":"ConfirmCard"},{"name":"SettingsNav"},{"name":"SystemBadge"},{"name":"BadgeChip"},{"name":"IndicatorBadge"},{"name":"CardIndicator"},{"name":"HoldRing"},{"name":"NoticeToast"},{"name":"Titlebar"},{"name":"RemoteArt"},{"name":"BannerArt"},{"name":"FailedHint"},{"name":"FailureLog"},{"name":"SearchToggle"},{"name":"ResultsBar"},{"name":"NoResults"},{"name":"ModShelf"},{"name":"Highlight"}]} */
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
    failed: ["danger", "alert", "Failed"], available: ["none", null, "Not installed"]
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
    var tone = { installed: "ok", update: "warn", installing: "info", failed: "danger", available: "none" }[p.status || "available"];
    return h("div", { className: cx("rc rc-row", p.selected && "sel", p.hover && "is-hover", p.size) }, h("span", { className: "ic" }), h("span", { className: "nm" }, p.title), p.version && h("span", { className: "rc-mono", style: { color: "var(--ink-subtle)" } }, p.version), h("span", { className: "dot " + tone, title: p.status }));
  }
  function HeroHeader(p) {
    var inst = p.status === "installed" || p.status === "update";
    return h("div", { className: cx("rc rc-hero", p.narrow && "narrow") },
      h("div", { className: "art" }, h(window.Reclaw.BannerArt, { seed: p.title, kind: p.banner, tag: p.bannerTag, strip: p.narrow ? 74 : 82 }), h("div", { className: "ttl" }, h("div", { className: "rc-eyebrow", style: { color: "var(--accent)" } }, p.project), h("h1", null, p.title))),
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
      mode === "top" && h("span", { style: { flex: 1 } }), mode === "top" && h(window.Reclaw.SearchToggle, null));
  }
  function InstallDialog(p) {
    return h("div", { className: "rc rc-scrim" }, h("div", { className: "rc-dialog" },
      h("div", { className: "hd" }, "Install " + (p.title || "Game")),
      h("div", { className: "bd" },
        h("div", { className: "rc-field" }, h("span", { className: "lb" }, "Install location"), h("div", { className: "rc-search" }, h(Icon, { name: "folder", size: 16 }), h("span", { className: "v rc-mono" }, "~/Reclaw/Apps/" + (p.slug || "game")))),
        h("div", { className: "rc-opt" }, h("span", null, "Keep pre-release builds"), h(Switch, { on: false }))),
      h("div", { className: "ft" }, h(Button, { variant: "ghost" }, "Cancel"), h(Button, { variant: "install", icon: "download" }, "Install"))));
  }

  /* Showcase only (not in the bundle header): the three layout classes composed from the parts above. */
  var GAMES = [["Starfall 64", "N64Recomp", "installed", "v1.4.2"], ["Skyward Quest", "Zelda-style port", "update", "v0.9.1"], ["Kart Ruins", "N64Recomp", "available", "v0.3.0"], ["Dino Rush", "PS2 recomp", "available", ""], ["Moon Garden", "GBA recomp", "failed", "v2.0.0"], ["Tide Racer", "N64Recomp", "installed", "v1.0.0"]];
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
    var g = p.game, badge = p.running ? h(StatusBadge, { kind: "installed", label: "Running" }) : h(StatusBadge, { kind: g.status, label: undefined });
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
  var DGAMES = [["Starfall 64", "installed"], ["Skyward Quest", "update"], ["Tide Racer", "installed"], ["Kart Ruins", "available"], ["Dino Rush", "available"]];
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
/* surfaces:start */
(function () {
  var R = window.React, h = R.createElement, RC = window.Reclaw;
  var cx = function () { return Array.prototype.filter.call(arguments, Boolean).join(" "); };
  var ChevR = function (s) { return h(RC.Icon, { name: "chevron", size: s || 20 }); };
  var Svg = function (d, size) { return h("svg", { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true }, h("path", { d: d })); };

  /* Scales a fixed-size frame so wide screens fit a preview card. */
  function Scaled(p) {
    var s = p.scale || 1;
    return h("div", { style: { width: p.w * s, height: p.h * s, flex: "none", outline: "1px solid var(--line-strong)", overflow: "hidden" } },
      h("div", { style: { width: p.w, height: p.h, transform: "scale(" + s + ")", transformOrigin: "top left" } }, p.children));
  }

  /* SettingRow: label (+ one-line description) left, control right. Heights match deck-settings-row-h (+72 for a text row). */
  function SettingRow(p) {
    var c = p.control, right = null;
    if (c === "toggle") right = h("span", { className: cx("sf-sw", p.on && "on") }, h("i"));
    if (c === "value") right = h("span", { className: "sf-val pick" }, p.value, Svg("M6 9l6 6 6-6", 18));
    if (c === "info") right = h("span", { className: "sf-val info" }, p.value);
    var top = h("div", { style: { display: "flex", alignItems: "center", gap: 16, flex: 1, minWidth: 0 } },
      h("div", { className: "l" }, h("div", { className: cx("t", p.danger && "danger") }, p.label), p.desc && h("div", { className: "d" }, p.desc)), right);
    return h("div", { className: cx("sf sf-row", c === "text" && "text", p.focus && "focus") }, top,
      c === "text" && h("div", { className: cx("sf-input", p.typing && "focus") }, p.value || h("span", { style: { color: "var(--ink-subtle)" } }, p.placeholder), p.typing && h("span", { className: "caret" })));
  }

  /* FullScreenPage: Back + title, scrolling body, footer actions, hint bar (Deck). Keyboard avoidance and the short-window header
     follow surface-short-h and the 40% rule from the contract. */
  function FullScreenPage(p) {
    var w = p.w, hh = p.h, kb = p.keyboard || 0;
    var compact = kb > hh * 0.4, head = compact ? 48 : 64, short = hh < 600, up = kb > 0;
    var actions = p.actions && h("div", { style: { display: "flex", gap: 12 } }, p.actions);
    return h("div", { className: cx("sf sf-page", p.deck && "deck"), style: { width: w, height: hh } },
      h("div", { className: "sf-head", style: { height: head } }, h("span", { className: "sf-back" }, Svg("M15 6l-6 6 6 6", 20), "Back"),
        h("div", { className: "sf-title", style: compact ? { fontSize: 20 } : null }, p.title), short && !up && actions),
      h("div", { className: "sf-body" }, p.fixed ? p.children : h("div", { className: "sf-col", style: { maxWidth: p.wide ? "none" : 720 } }, p.children,
        up && h("div", { style: { height: kb, flex: "none" } }))),
      !up && !short && actions && h("div", { className: "sf-foot" }, actions),
      !up && p.hints && h("div", { className: "sf-hintwrap" }, p.hints),
      up && h("div", { className: "sf-kb", style: { height: kb } }, "On-screen keyboard · " + kb + "px"));
  }

  /* ModalMenu: darkened screen, title above, columns side by side; the focused row is inverted. rows: [label, {chev,sep,focus,open,off}] */
  function MenuColumn(p) {
    return h("div", { className: "sf-col-menu", style: p.colw ? { width: p.colw } : null }, p.rows.map(function (r, i) {
      var o = r[1] || {};
      return h("div", { key: i, className: cx("sf-mrow", o.sep && "sep", o.focus && "focus", o.open && "open", o.off && "off") }, h("span", { className: "l" }, r[0]), o.chev && ChevR(20));
    }));
  }
  function ModalMenu(p) {
    return h("div", { className: "sf sf-room", style: { width: p.w, height: p.h } },
      h("div", { className: "under" }, p.under),
      h("div", { className: "dim" }),
      h("div", { className: "stack" }, h("div", { className: "sf-menu-title" }, p.title), h("div", { className: "sf-cols" }, p.columns.map(function (c, i) { return h(MenuColumn, { key: i, rows: c, colw: p.colw }); }))));
  }

  function ConfirmCard(p) {
    return h("div", { className: "sf sf-confirm" }, h("div", { className: "sf-menu-title" }, p.title), h("div", { className: "m" }, p.message),
      h("div", { className: "a" }, h(RC.Button, { variant: "secondary", size: "lg" }, "Cancel"), h(RC.Button, { variant: "danger", size: "lg" }, p.action)));
  }

  function Nav(p) {
    return h("div", { className: "sf sf-nav", style: p.full ? { width: "100%" } : null }, p.items.map(function (t, i) { return h("div", { key: i, className: cx("sf-navitem", i === p.sel && "sel", i === p.focus && "focus") }, t); }));
  }

  Object.assign(window.Reclaw, { SettingRow: SettingRow, FullScreenPage: FullScreenPage, ModalMenu: ModalMenu, ConfirmCard: ConfirmCard, SettingsNav: Nav, Scaled: Scaled });
})();
/* surfaces:end */
/* activity:start */
(function () {
  var R = window.React, h = R.createElement, RC = window.Reclaw;
  var cx = function () { return Array.prototype.filter.call(arguments, Boolean).join(" "); };
  var Svg = function (d, size, w) { return h("svg", { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: w || 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true }, h("path", { d: d })); };
  var PATH = {
    clock: "M12 7v5l3 2M12 3a9 9 0 100 18 9 9 0 000-18z", queue: "M12 4v12m0 0l-4-4m4 4l4-4M5 20h14", package: "M4 7l8-4 8 4v10l-8 4-8-4zM4 7l8 4m0 0l8-4m-8 4v10",
    download: "M12 4v11m0 0l-4-4m4 4l4-4M5 20h14", check: "M5 12l5 5 9-10", alert: "M12 8v5m0 4h.01M10.3 4l-8 14a2 2 0 001.7 3h16a2 2 0 001.7-3l-8-14a2 2 0 00-3.4 0z",
    mods: "M10 4h4v3a2 2 0 104 0V4h2v6h-3a2 2 0 100 4h3v6h-6v-3a2 2 0 10-4 0v3H4v-6h3a2 2 0 100-4H4V4z",
    min: "M5 12h14", max: "M5 5h14v14H5z", restore: "M8 8V4h12v12h-4M4 8h12v12H4z", close: "M6 6l12 12M18 6L6 18"
  };

  /* Same table as components::indicator_look. */
  var LOOK = {
    update: ["upd", "download"], queued: ["que", "clock"], downloading: ["dl", "queue"], installing: ["ins", "package"],
    done: ["done", "check"], failed: ["fail", "alert"], mods: ["mods", "mods"]
  };
  var STRIP = { update: null, queued: "var(--ink-muted)", downloading: "var(--accent)", installing: "var(--info)", done: "var(--ok)", failed: null, mods: "var(--accent)" };

  function SystemBadge(p) { return h("span", { className: cx("ac ac-sys", p.overArt && "art", p.large && "lg") }, p.short); }

  function BadgeChip(p) {
    var dark = p.darkText;
    return h("span", { className: "ac ac-chip" }, p.label && h("span", { className: "l" }, p.label),
      h("span", { style: { background: p.color, color: dark ? "#111418" : "#fff" } }, p.message));
  }

  function IndicatorBadge(p) {
    var l = LOOK[p.kind];
    return h("span", { className: cx("ac ac-ind", l[0], p.large && "lg", p.filled && "filled") }, Svg(PATH[l[1]], p.large ? 20 : 14), p.label);
  }

  function CardIndicator(p) {
    var color = STRIP[p.kind], bar = null;
    if (p.kind === "done") bar = h("div", { className: "ac-strip" }, h("i", { style: { width: "100%", background: color } }));
    else if (color && p.progress != null) bar = h("div", { className: "ac-strip" }, h("i", { style: { width: Math.round(p.progress * 100) + "%", background: color } }));
    else if (color) bar = h("div", { className: "ac-strip slide" }, h("i", { style: { background: color } }));
    return h("div", { className: "ac ac-card" }, h("span", { className: "ph" }, "CAPSULE 3:4"),
      h("div", { className: "tl" }, h(IndicatorBadge, { kind: p.kind, label: p.label, large: true, filled: true })), bar);
  }

  /* The faces a hold prompt can show, by controller. West is X on Xbox and Steam Deck, a square on PlayStation, Y on Nintendo. */
  var FACE = {
    xbox: { x: "X", y: "Y" }, steamdeck: { x: "X", y: "Y" }, nintendo: { x: "Y", y: "X" }, keyboard: { x: "X", y: "Y" },
    playstation: { x: PATH.max, y: "M12 5l8 14H4z" }
  };
  function HoldRing(p) {
    var r = 20, c = 2 * Math.PI * r, steps = Math.round(Math.max(0, Math.min(1, p.progress || 0)) * 120) / 120;
    var kb = p.kind === "keyboard", face = (FACE[p.kind || "xbox"] || FACE.xbox)[p.button || "x"];
    return h("span", { className: "ac ac-ring" },
      h("svg", { className: "r", width: 44, height: 44, viewBox: "0 0 44 44" },
        h("circle", { cx: 22, cy: 22, r: r, fill: "none", stroke: "var(--line-strong)", strokeWidth: 4 }),
        steps > 0 && h("circle", { cx: 22, cy: 22, r: r, fill: "none", stroke: "var(--accent)", strokeWidth: 4, strokeLinecap: "round", strokeDasharray: c, strokeDashoffset: c * (1 - steps) })),
      h("span", { className: cx("dk dk-glyph", kb && "key") }, p.kind === "playstation" ? Svg(face, 16) : face));
  }

  var NOTICE = { update: ["upd", "download"], finished: ["done", "check"], failed: ["fail", "alert"] };
  function NoticeToast(p) {
    var l = NOTICE[p.tone || "update"], held = p.holding;
    var prompt = function (id, label) {
      var on = held === id;
      return h("span", { className: cx("ac-hold", on && "on") }, h(HoldRing, { kind: p.kind, button: id, progress: on ? p.progress : 0 }), label);
    };
    return h("div", { className: cx("ac ac-toast", l[0]), role: "status" },
      h("div", { className: "hd" }, h("span", { className: cx("ac-ind", "lg", l[0]) }, Svg(PATH[l[1]], 28)), h("div", null, h("div", { className: "tt" }, p.title), h("div", { className: "bd" }, p.body))),
      h("div", { className: "hs" }, prompt("x", "Hold for details"), prompt("y", "Hold to dismiss all")));
  }

  function Titlebar(p) {
    var b = function (name, d, cls, hover) { return h("span", { className: cx("ac-wbtn", cls, hover && "hover") }, Svg(d, 14)); };
    return h("div", { className: "ac ac-titlebar", style: { width: p.width || "100%" } },
      h("div", { className: "drag" }, h("b", null, "Reclaw"), h("span", null, "/ " + (p.page || "Library"))),
      b("min", PATH.min, "", p.hover === "min"), b("max", p.maximized ? PATH.restore : PATH.max, "", p.hover === "max"), b("close", PATH.close, "close", p.hover === "close"));
  }

  function RemoteArt(p) {
    var s = p.state || "loading";
    return h("div", { className: cx("ac ac-art", s), style: { width: p.w || 256, height: p.h || 144 } }, h("span", { className: "tag" }, p.tag || "SCREENSHOT"));
  }

  // FNV-1a over the UTF-8 bytes, as the app does (reclaw_ui::banner::hue_of): the same game gets the same colour in the app and here.
  function hueOf(seed) {
    var bytes = new TextEncoder().encode(String(seed).trim().toLowerCase()), hash = BigInt("0xcbf29ce484222325"), prime = BigInt("0x100000001b3"), mask = (BigInt(1) << BigInt(64)) - BigInt(1);
    for (var i = 0; i < bytes.length; i++) hash = ((hash ^ BigInt(bytes[i])) * prime) & mask;
    return Number(hash % BigInt(360));
  }

  function BannerArt(p) {
    var hue = hueOf(p.seed || "game"), kind = p.kind || "generated";
    var wash = "linear-gradient(135deg,color-mix(in srgb,hsl(" + hue + " 62% 46%) 62%,var(--bg-base)),color-mix(in srgb,hsl(" + (hue + 38) % 360 + " 50% 40%) 16%,var(--bg-base)))";
    var style = Object.assign({ "--bn-strip": (p.strip == null ? 0 : p.strip) + "px" }, p.height ? { height: p.height } : null, p.style);
    if (kind === "picture") {
      return h("div", { className: cx("rc-banner pic", p.className), style: style }, h("span", { className: "tag" }, p.tag || "CATALOG BANNER"));
    }
    var mark = (p.icon || String(p.seed || "?").trim().charAt(0) || "?").toUpperCase();
    return h("div", { className: cx("rc-banner gen", p.className), style: Object.assign({ background: wash }, style) },
      h("div", { className: "bn-room" }, h("div", { className: "bn-glow", "aria-hidden": "true" }), h("div", { className: "bn-icon", style: { "--bn-hue": hue } }, mark)));
  }

  Object.assign(window.Reclaw, { SystemBadge: SystemBadge, BadgeChip: BadgeChip, IndicatorBadge: IndicatorBadge, CardIndicator: CardIndicator, HoldRing: HoldRing, NoticeToast: NoticeToast, Titlebar: Titlebar, RemoteArt: RemoteArt, BannerArt: BannerArt });
})();
/* activity:end */
/* failure:start */
(function () {
  var R = window.React, h = R.createElement;
  var cx = function () { return Array.prototype.filter.call(arguments, Boolean).join(" "); };
  var ALERT = "M12 8v5m0 4h.01M10.3 4l-8 14a2 2 0 001.7 3h16a2 2 0 001.7-3l-8-14a2 2 0 00-3.4 0z";
  var FOLDER = "M3 7a2 2 0 012-2h4l2 2h8a2 2 0 012 2v8a2 2 0 01-2 2H5a2 2 0 01-2-2z";
  var Svg = function (d, size) { return h("svg", { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true, "data-lucide": d === ALERT ? "triangle-alert" : "folder-open" }, h("path", { d: d })); };

  /* Same rule as activity::short_reason: one line, 80 characters, cut at a word. */
  function shortReason(text) {
    var line = String(text || "").split("\n")[0].trim();
    if (line.length <= 80) return line;
    var cut = line.slice(0, 77), at = cut.lastIndexOf(" ");
    return (at >= 38 ? cut.slice(0, at) : cut).replace(/[ ,;:.]+$/, "") + "...";
  }

  /* A "Failed" indicator with its tooltip shown: the Downloads label (kind "label") or the status badge (kind "badge"). */
  function FailedHint(p) {
    var target = p.kind === "badge"
      ? h(window.Reclaw.StatusBadge, { kind: "failed" })
      : h("span", { className: "fl-label" }, Svg(ALERT, 12), "Failed");
    return h("span", { className: "fl fl-anchor" }, target, p.open !== false && h("span", { className: "fl-tip", role: "tooltip" }, p.earlier ? "The last install did not finish. Its log is in the log folder." : shortReason(p.reason)));
  }

  /* The dialog a press opens: whole reason, details, the job's log, Open log folder. */
  function FailureLog(p) {
    var tone = function (line) { var lvl = line.trim().split(/\s+/)[1]; return lvl === "ERROR" ? "error" : lvl === "WARN" ? "warn" : null; };
    return h("div", { className: "fl fl-dialog", role: "dialog" },
      h("h3", null, (p.title || "Game") + " failed"),
      h("div", { className: "fl-reason" }, p.reason),
      (p.details || []).map(function (d, i) { return h("div", { key: i, className: "fl-detail" }, d); }),
      h("div", { className: "fl-cap" }, "Log"),
      (p.log && p.log.length)
        ? h("div", { className: "fl-log" }, p.log.map(function (l, i) { return h("div", { key: i, className: cx(tone(l)) }, l); }))
        : h("div", { className: "fl-log" }, h("div", null, "Nothing was recorded for this job. The log file may have more.")),
      h("div", { className: "fl-actions" }, h("span", { className: "fl-btn" }, Svg(FOLDER, 16), "Open log folder"), h("span", { className: "fl-btn secondary" }, "Close")));
  }

  Object.assign(window.Reclaw, { FailedHint: FailedHint, FailureLog: FailureLog });
})();
/* failure:end */
/* search:start */
(function () {
  var R = window.React, h = R.createElement;
  var cx = function () { return Array.prototype.filter.call(arguments, Boolean).join(" "); };
  var Svg = function (d, size, lucide) { return h("svg", { width: size, height: size, viewBox: "0 0 24 24", fill: "none", stroke: "currentColor", strokeWidth: 2, strokeLinecap: "round", strokeLinejoin: "round", "aria-hidden": true, "data-lucide": lucide }, h("path", { d: d })); };
  var MAG = "M11 18a7 7 0 100-14 7 7 0 000 14zm9 3l-4.5-4.5", X = "M6 6l12 12M18 6L6 18", CHEV = "M9 6l6 6-6 6", BACK = "M15 6l-6 6 6 6";
  var SHELF = { installed: ["M5 12l5 5 9-10", "check"], popular: ["M12 4v11m0 0l-4-4m4 4l4-4M5 20h14", "download"], rated: ["M12 3l2.7 5.6 6.1.9-4.4 4.3 1 6.1L12 17l-5.4 2.9 1-6.1-4.4-4.3 6.1-.9z", "star"], updated: ["M20 11a8 8 0 10-2.3 5.7M20 4v7h-7", "refresh-cw"], newest: ["M12 4a8 8 0 100 16 8 8 0 000-16zm0 4v4l3 2", "clock"] };

  /* The magnifier that slides open to the left. Its own box never changes size: the bar is absolute, anchored at the right.
     Interactive in the preview: press to open, Enter or the magnifier to send, click away to close (what was typed is kept). */
  function SearchToggle(p) {
    var st = R.useState(!!p.open), open = st[0], setOpen = st[1];
    var vt = R.useState(p.value || ""), value = vt[0], setValue = vt[1];
    var qt = R.useState(p.active ? "zelda" : ""), query = qt[0], setQuery = qt[1];
    var input = R.useRef(null);
    R.useEffect(function () { if (open && input.current) input.current.focus(); }, [open]);
    var width = open ? (p.expanded || 320) : (p.size === "touch" ? 44 : 32);
    var send = function () { setQuery(value.trim()); setOpen(false); if (p.onSubmit) p.onSubmit(value.trim()); };
    return h("span", { className: cx("sr sr-slot", p.size) },
      h("span", { className: cx("sr-bar", open && "open"), style: { width: width }, onClick: function () { if (!open) setOpen(true); } },
        h("input", { ref: input, value: value, tabIndex: open ? 0 : -1, placeholder: open ? (p.placeholder || "Search your library") : "", onChange: function (e) { setValue(e.target.value); }, onBlur: function () { if (!p.pinned) setOpen(false); }, onKeyDown: function (e) { if (e.key === "Enter") send(); if (e.key === "Escape") e.target.blur(); } }),
        h("span", { className: "sr-mag", onMouseDown: function (e) { if (open) { e.preventDefault(); send(); } } }, Svg(MAG, 16, "search"))),
      !open && query && h("span", { className: "sr-dot", title: "A search is running" }));
  }

  /* Words of the query marked in a title, as search::highlight does. */
  function Highlight(p) {
    var terms = String(p.query || "").toLowerCase().split(/\s+/).filter(Boolean), text = String(p.text), lower = text.toLowerCase(), marks = [];
    terms.forEach(function (t) { var at = 0, i; while ((i = lower.indexOf(t, at)) >= 0) { marks.push([i, i + t.length]); at = i + t.length; } });
    marks.sort(function (a, b) { return a[0] - b[0]; });
    var out = [], pos = 0;
    marks.forEach(function (m) { if (m[0] < pos) { if (m[1] > pos) { out[out.length - 1].e = m[1]; pos = m[1]; } return; } if (m[0] > pos) out.push({ s: pos, e: m[0], hit: false }); out.push({ s: m[0], e: m[1], hit: true }); pos = m[1]; });
    if (pos < text.length) out.push({ s: pos, e: text.length, hit: false });
    return h("span", null, out.map(function (r, i) { return h("span", { key: i, className: r.hit ? "sr-hit" : null }, text.slice(r.s, r.e)); }));
  }

  function ResultsBar(p) {
    var n = p.found === 1 ? "1 result" : p.found + " results";
    return h("div", { className: "sr sr-results" }, Svg(MAG, 16, "search"), h("b", null, n + " for “" + p.query + "”"), h("span", { className: "where" }, p.narrow ? "" : (p.place || "in mods")), h("span", { className: "clear" }, Svg(X, 16, "x"), p.narrow ? null : "Clear search"));
  }

  function NoResults(p) {
    return h("div", { className: "sr sr-none" }, Svg(MAG, 28, "search"), h("h4", null, "Nothing " + (p.place || "in the catalog") + " matches “" + p.query + "”"), h("p", null, p.hint || "Try other words, or another platform."), h(window.Reclaw.Button, { icon: "x" }, "Clear search"));
  }

  function ModRow(p) {
    var m = p.mod;
    return h("div", { className: "ms-row" }, h("div", { className: "txt" }, h("span", { className: "t" }, p.query ? h(Highlight, { text: m.title, query: p.query }) : m.title), h("span", { className: "m" }, m.meta), h("span", { className: "s" }, m.summary)), h(window.Reclaw.Button, { variant: m.installed ? "secondary" : "primary", icon: m.installed ? "x" : "download" }, m.installed ? "Remove" : "Install"));
  }

  /* One shelf of the Mods tab: icon, title, count, what the order is, a few mods, and Show all (or All shelves when opened). */
  function ModShelf(p) {
    var s = SHELF[p.kind || "popular"];
    var mods = p.mods || [];
    return h("div", { className: "sr ms" },
      h("div", { className: "ms-head" }, h("span", { className: "ms-ico" }, Svg(s[0], 16, s[1])), h("span", { className: "ms-title" }, h("b", null, p.title), h("span", { className: "n" }, p.count != null ? p.count : mods.length), h("span", { className: "note" }, p.note)),
        p.opened ? h("span", { className: "ms-more" }, Svg(BACK, 16, "chevron-left"), "All shelves") : (p.count > mods.length && h("span", { className: "ms-more" }, Svg(CHEV, 16, "chevron-right"), "Show all " + p.count))),
      h("div", { className: "ms-grid", style: { "--cols": p.columns || 2 } }, mods.map(function (m, i) { return h(ModRow, { key: i, mod: m, query: p.query }); })),
      p.pages > 1 && h("div", { className: "ms-pager" }, h(window.Reclaw.Button, { icon: "chevron", disabled: !p.page }, "Previous"), h("span", { className: "mid" }, "Page " + ((p.page || 0) + 1) + " of " + p.pages + "  ·  " + p.range), h(window.Reclaw.Button, { icon: "chevron" }, "Next")));
  }

  Object.assign(window.Reclaw, { SearchToggle: SearchToggle, ResultsBar: ResultsBar, NoResults: NoResults, ModShelf: ModShelf, Highlight: Highlight });
})();
/* search:end */
