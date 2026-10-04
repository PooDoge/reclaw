Reclaw's Settings and an app's Properties are the same page built from a schema (`deck::settings`: sections, groups, rows). Rust: `reclaw_ui::deck::SettingsBody` inside a `FullScreenPage`.

**Layout.** At `deck-two-pane-min-w` (900) and wider: a section list `deck-settings-nav-w` (400) wide beside the rows, each scrolling on its own. Narrower: the section list alone, then a section's rows after a press; Back goes one step back, then leaves. The selected section has a blue wash, the focused one a raised fill with an accent bar.

**Rows** (`SettingRow`): `deck-settings-row-h` (68) tall, one line of label and one of description (truncated, so keep descriptions short). A toggle shows a switch, a choice shows its value and a chevron and opens a centered menu, info shows plain text, an action is the whole row (danger in red). A text row is 72px taller and holds an input: pressing it from a pad or keyboard starts typing, clearing the box (the old text returns if nothing is typed) because Freya's input cannot move its caret from outside; a tap or click keeps the text.

Layout and focus both read `section_slots`, so what is drawn and what can be focused cannot drift apart.
