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

  Object.assign(window.Reclaw, { SystemBadge: SystemBadge, BadgeChip: BadgeChip, IndicatorBadge: IndicatorBadge, CardIndicator: CardIndicator, HoldRing: HoldRing, NoticeToast: NoticeToast, Titlebar: Titlebar, RemoteArt: RemoteArt });
})();
