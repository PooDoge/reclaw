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
        h("input", { ref: input, value: value, tabIndex: open ? 0 : -1, placeholder: open ? (p.placeholder || "Search your library") : "", onChange: function (e) { setValue(e.target.value); }, onBlur: function () { setOpen(false); }, onKeyDown: function (e) { if (e.key === "Enter") send(); if (e.key === "Escape") e.target.blur(); } }),
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
