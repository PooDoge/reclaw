# Layout

Layout class comes from the container width, read once at the app root. Never branch on the OS or on a platform name.

| class | width | shell | library | grid |
| --- | --- | --- | --- | --- |
| wide | 1100px and up | top nav (`topbar-h` 40) | sidebar `sidebar-w` 240 beside the page | capsules `auto-fill`, `capsule-w` 168 |
| compact | 720 to 1099px | icon rail `rail-w` 64 | library is the capsule grid; list hidden | 4 fluid columns |
| phone | under 720px | bottom tabs `tabbar-h` 56 | capsule grid, 2 fluid columns, game page pushes over it | 2 fluid columns |

Density is separate. **Pointer**: rows `row-h` 32, buttons 32, body 14. **Touch / controller**: rows `row-h-touch` 48, buttons 48, `body-touch` 16, no hover-only affordances, every hit target at least `target-min` 44. Choose touch when the device reports touch or when a gamepad is the active input; a 1280x800 handheld is wide layout plus touch density.

## Behaviors by class

- **Game page.** Wide: hero fills the main column next to the sidebar. Compact: hero above the grid, collapsed bar. Phone: pushed page with a back button at the top, the install action pinned in a bar above the tabs.
- **Dialogs.** Wide and compact: centered `Popup`, 420px. Phone: bottom sheet.
- **Sidebar to rail.** Animates width 240 to 64 over 240ms; labels fade out first so no text is ever clipped.
- **Downloads.** One list at every width; the speed and detail line wrap under the name on phone.
- **Empty library.** Centered message plus an "Add from catalog" install-style button and an "Add folder" ghost button.

## Freya rules that bite here

- Every row that mixes a `Size::flex(1.)` child with a fixed sibling needs `.content(Content::Flex)` on the parent. Library rows, hero action bars and download rows all do.
- `ScrollView` has no padding: wrap it in a padded `rect()`. Hide rail scrollbars with `.show_scrollbar(false)`.
- Capsule grids on wide: use `VirtualScrollView` with fixed item size; a library can hold hundreds of entries.
