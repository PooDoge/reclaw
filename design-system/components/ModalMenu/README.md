The Options menu, in the shape of Big Picture's context menu. Rust: `reclaw_ui::surface::{ModalMenu, MenuState}`; the entries for an app are `app_menu::options_menu`.

**Centered** (touch, pad, phones): the screen darkens to a 85% scrim, the title sits above, the menu is centered. Rows are `deck-row-h` tall in columns `deck-menu-w` wide. The focused row is **inverted** (`ink` fill, `deck-bg` text), which reads at ten feet. `>` rows open a submenu in a second column to the right; the parent stays visible with its open row in `ink-muted`. Group separators are 4px gaps of `deck-bg`. Disabled rows use `ink-subtle`. When the window cannot fit two columns only the deepest level shows, and Back returns one level.

**Anchored** (pointer, desktop): a popover at the press point with a border and shadow, no scrim, 40px rows, 280px wide; it slides to stay on screen. A click outside dismisses it.

Behavior lives in `MenuState` (focus per level, open path, Back), tested without a window: Up and Down stop at the ends and skip disabled rows; Right or Confirm on `>` opens it; Left or Back closes one level; Back at the root closes the menu. Pickers behind a choice row use the same menu without a Cancel row.
