"""Adds the components that explain a failed job (the tooltip on "Failed" and the log dialog) to the design system: patches
components/bundle.{js,css} between marker comments and writes their README + preview files.
Safe to run again. Run from anywhere: `python3 design-system/tools/gen_failure.py`."""
import os, re, json
os.chdir(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
exec(open("tools/gen_activity.py").read().split("patch(\"components/bundle.css\"")[0].replace('NAMES = [', 'ACTIVITY_NAMES = ['))

NAMES = ["FailedHint", "FailureLog"]

patch("components/bundle.css", "/* failure:start */", "/* failure:end */", open("tools/failure.css").read())
patch("components/bundle.js", "/* failure:start */", "/* failure:end */", open("tools/failure.js").read())
js = open("components/bundle.js").read()
first, rest = js.split("\n", 1)
m = re.match(r"(/\* @ds-bundle: )(\{.*\})( \*/)", first)
manifest = json.loads(m.group(2))
manifest["components"] = [c for c in manifest["components"] if c["name"] not in NAMES]
manifest["components"] += [{"name": n} for n in NAMES]
open("components/bundle.js", "w").write(m.group(1) + json.dumps(manifest, separators=(",", ":")) + m.group(3) + "\n" + rest)

def readme(name, text):
    os.makedirs(f"{P}/{name}", exist_ok=True)
    open(f"{P}/{name}/README.md", "w").write(text.strip() + "\n")

REASON = "This release's download for Linux-X64 is a Windows installer, which Reclaw does not install. Pick another release."
preview("FailedHint", "Progress", 200, "Hover a Failed label or badge for the reason in one line; press it for the log",
 f'''h("div",{{className:"col"}},
  h("div",null,h("div",{{className:"cap"}},"Downloads row label, hovered"),h(R.FailedHint,{{kind:"label",reason:{json.dumps(REASON)}}})),
  h("div",null,h("div",{{className:"cap"}},"Status badge (Library hero, game page, capsule), hovered"),h(R.FailedHint,{{kind:"badge",reason:"Release asset not found. Check the repository or choose another version."}})),
  h("div",null,h("div",{{className:"cap"}},"Failed before the last restart"),h(R.FailedHint,{{kind:"badge",earlier:true}})))''')
readme("FailedHint", """
What every desktop "Failed" indicator does. Rust: `reclaw_ui::components::FailedHint`, used by `DownloadItem` (the stage label), `StatusBadge` (`.failure(text)` and `.on_failure(handler)`), `HeroHeader` (`.failure(text, handler)`) and `GameCapsule` (`.failure(text)`, tooltip only). The text and the target come from `reclaw_ui::activity::{hint_for_game, hint_for_activity}` (pure, tested).

* **Hover:** Freya's `TooltipContainer` after its 500 ms delay, one line: the reason cut to 80 characters at a word, with "..." (`short_reason`). Below the badge; to the left of a Downloads label, which sits at the right edge of its row.
* **Press:** opens `FailureLog` for the job. A capsule's badge has the tooltip only: pressing a capsule opens the game, and its page's badge opens the log.
* **Earlier run:** a game that says Failed after a restart has no job left on the board. The tooltip says the last install did not finish and its log is in the log folder; a press opens the log folder (`Effect::OpenLogFolder`).
* The label gains the triangle-alert icon so it reads as something to act on, not only a word. Pointer cursor when pressable; an accessible name "Failed: <reason>. Show the log".

Deck mode has no hover. Its failure notice carries the same details; the log view is not on Deck yet.
""")
preview("FailureLog", "Progress", 400, "Why a job failed: the whole reason, what to do, and the lines it wrote to the log",
 f'''h(R.FailureLog,{{title:"Moon Garden v2.0.1",reason:{json.dumps(REASON)},details:["The log has more: ~/.local/state/reclaw/logs/reclaw.log"],log:["05:42:00.101  INFO reclaw_app::host::install::jobs: install started app=5 title=\\"Moon Garden\\" repo=\\"example/moon-garden\\" host=GitHub platform=Linux-X64","05:42:00.388 DEBUG reclaw_net::net: fetched url=api.github.com/repos/example/moon-garden/releases status=200","05:42:00.405  WARN reclaw_app::host::install::jobs: the install failed app=5 error=This release's download for Linux-X64 is a Windows installer"]}})''')
readme("FailureLog", """
The dialog a press on "Failed" opens. Rust: `FailureLogView` in `reclaw_ui::desktop::dialogs::failure_log`, a `Dialog` of kind Form with `.wide(true)`: a 760px popup on desktop, a full-screen page with Back on touch. Data: `reclaw_ui::activity::report(board, id)`.

* Title "<job> failed"; the whole reason in `body` `ink`; each detail (the cause, what to do, where the log file is) in `meta` `ink-muted`.
* **Log:** the lines the job wrote while it ran (`Activity::log`, recorded by `reclaw_log::record` around the install, ADR 0019), oldest first, `mono` on `bg-deep` in a `line` box. Warnings in `warn`, errors in `danger` (`line_tone` reads the level column, not the words). The box grows with the lines up to 320px, then scrolls. At most the last 400 lines; when more were written the first line says how many were dropped.
* Same level and redaction as the log file: Settings, Diagnostics, Log detail "Detailed" records more. No token is ever in a line.
* Actions: "Open log folder" (ghost, folder-open) for the whole file, "Close" (secondary). Back and Escape close it before the page.

Don't: show it for a running or finished job (there is no report); put the log anywhere a stranger can read it without the person choosing to share it.
""")
