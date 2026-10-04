/* @ds-bundle: {"format":4,"namespace":"Reclaw","components":[{"name":"Button"},{"name":"Chip"},{"name":"StatusBadge"},{"name":"Switch"},{"name":"SearchField"},{"name":"GameCapsule"},{"name":"LibraryRow"},{"name":"HeroHeader"},{"name":"DownloadItem"},{"name":"Nav"},{"name":"InstallDialog"}]} */
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
    folder: "M3 6h6l2 2h10v11H3z", chevron: "M9 6l6 6-6 6", refresh: "M20 11a8 8 0 10-2.3 5.7M20 4v7h-7", file: "M6 3h8l4 4v14H6zM14 3v4h4"
  };
  var LUCIDE = { play: "play", download: "download", check: "check", alert: "triangle-alert", x: "x", search: "search", library: "library-big", catalog: "store", mods: "puzzle", queue: "arrow-down-to-line", settings: "settings", folder: "folder-open", chevron: "chevron-right", refresh: "refresh-cw", file: "file" };
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

  window.Reclaw = { Button: Button, Chip: Chip, StatusBadge: StatusBadge, Switch: Switch, SearchField: SearchField, GameCapsule: GameCapsule, LibraryRow: LibraryRow, HeroHeader: HeroHeader, DownloadItem: DownloadItem, ProgressBar: ProgressBar, Nav: Nav, InstallDialog: InstallDialog, Icon: Icon, Art: Art, LibraryScreen: LibraryScreen };
})();
