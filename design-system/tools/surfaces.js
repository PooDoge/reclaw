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
