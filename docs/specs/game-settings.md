# Game settings

- last-verified: 2026-10-05
- owner-paths: reclaw-games/src/settings/**, reclaw-games/src/project.rs, reclaw-games/src/fixtures.rs

Launch settings (window mode, resolution, upscaling ...) that Reclaw can pass to a game. The crate is `reclaw-games`; the UI is in
`reclaw-ui` and only displays what this crate says is supported.

## The rule

**A setting is shown for a game only if the game declares it and the display can honor it.** A setting that is displayed and not
applied is worse than one that does not exist. A game that declares nothing shows no launch settings, not an empty page of
disabled rows.

## Model

* `SettingKey`: the launcher's fixed catalog (11 keys in 3 groups, `SettingKey::ALL`). Adding a key is a code change: it needs a
  label, a value shape in `standard_kind`, and an answer for every `DisplayServer`.
* `Capabilities` (per game, written in the catalog): a list of `Binding { key, constraint, outputs }`.
  * `Constraint` narrows the standard range for this game: `Choices` (a subset of choice ids), `Range`, `Sizes` (the only
    resolutions the game accepts).
  * `Output` applies a chosen value: `Args`, `Env`, or `Config` (an edit to a file in the game's install, config or data directory).
    `when` limits an output to one value. Templates use `{value}`, `{width}`, `{height}`, `{mapped}`; `map` rewrites a value to the
    game's own word before it becomes `{mapped}`.
* `SettingsLayer`: stored user choices, a map from key to value. Two kinds: the global defaults, and one per game (overrides).
  **Absent means not set at this level, never off.**
* `DisplayEnvironment`: what the host says the display is (`DisplayServer`, monitors). The UI never probes the system.

## Resolution (`effective`, `plan`)

For each setting in `supported(caps, env)`, in `SettingKey::ALL` order:

1. The game override, if it `adapt`s to the spec, wins: `Source::GameOverride`.
2. Else the user default, if it adapts: `Source::UserDefault`.
3. Else `Source::GameDefault` with no value: the game decides and the launcher passes nothing.

A value that does not adapt is **ignored, not replaced** (an override that no longer fits falls through to the default; a default
that does not fit this game is skipped for this game). The one exception is below.

`adapt(spec, value)`:

| Kind | Rule |
|---|---|
| Bool | `Bool` only |
| Int | `Int` clamped to `min..=max`, then snapped to the nearest `min + n*step` |
| Choice | `Choice(id)` only if `id` is an option |
| Size | `Size(s)` as is if listed; else the listed size with the largest area not above `s` (never larger than the user asked for); `None` if all are larger. `Native` only if the kind allows it |

`plan` renders the outputs of every setting that has a value, in catalog order. An output whose `when` differs from the value, or whose
template needs data we lack (`{width}` for `Native` with no known monitor, `{mapped}` with no map entry), is skipped, never emitted
half-built. Only the four placeholders are replaced. Config edits are grouped per file (first-seen order); a later edit to the same path
wins. A typed config value follows `ValueType` (`Auto`: `true`/`false` become booleans, whole numbers integers, anything else text).

## Display environment

| Key | Rule |
|---|---|
| Window mode | Wayland: windowed and borderless only (the compositor owns exclusive fullscreen). Gamescope: not offered. Others: all three |
| Resolution | Standard list up to the primary monitor's native size (all of it when no monitor is known), plus Native |
| Monitor | Offered only with two or more monitors, never under gamescope; one choice per monitor, labeled `name (WxH)` |
| Other keys | Offered everywhere |

Where the environment comes from: the window layer reads winit's monitors once and `window::monitors::environment` builds the
`DisplayEnvironment` (`docs/specs/window.md`); the UI never probes the system itself. A monitor's id is its connector name made unique,
or `monitor-N` when the system gave none. What a game's saved Monitor choice shows after that monitor is unplugged is not covered by a
test, and how many monitors Wayland compositors report to winit on a real Bazzite session is unverified.

## Config files

`apply_edits(format, text, edits)` is pure: it returns the new text and leaves everything else as it was.

* JSON: keys keep their order; indentation is detected from the file (tab, 2, 4; default 2); paths are dotted and a numeric segment
  indexes an array; missing objects are created; a file that does not parse is an error (comments are not JSON).
* TOML: comments and layout survive (`toml_edit`); paths are dotted; no array indexes.
* INI: `section.key`, split at the last dot, no dot means before any section; comments (`;` `#`) and the `key=value` / `key = value`
  spacing of an existing line survive; a missing key is added at the end of its section; a missing section at the end of the file.
* Key-value: the whole path is the key; the separator of an existing line is kept; a new key copies the first key line's separator
  (a single space when there is none).

Safety: `Bases::resolve` refuses an absolute path or any `..`. `write_atomic` writes a temporary file next to the target and renames
it, and keeps the first original as `<name>.reclaw-orig`. `LaunchPlan::apply_config` stops at the first error and leaves that file
untouched.

## Known edges

* **`.reclaw-orig` is only made when the file existed.** A file Reclaw creates has no original to keep (tested), so there is nothing to
  restore it to except deleting it.
* **A symlinked config file is replaced by a regular file.** `write_atomic` renames a temporary file onto the path, which replaces the
  link and leaves its target as it was. Read from the code; no test covers it. A game whose config is a link into a shared folder will
  stop following it after the first apply.
* **Windows is unverified.** Replacing an open file with a rename behaves differently there.

## Not built

Detecting what a game supports from the game itself (capabilities are declared in the catalog), per-monitor refresh rate and HDR,
and applying settings to a game that reads a binary config.
