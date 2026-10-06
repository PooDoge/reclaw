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
