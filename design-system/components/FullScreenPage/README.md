The page every form, settings screen and install flow becomes on a phone, a short touch screen and in Deck mode, in the pattern of a mobile settings page. Rust: `reclaw_ui::surface::FullScreenPage`; `Dialog` chooses it for you.

**Anatomy.** Header (`surface-header-h` 64, Back button + title), body, footer (`surface-footer-h` 88, actions right-aligned), hint bar in Deck mode. The body is a centered column no wider than `surface-max-w` (720); two-pane pages use the full width.

**When it is used** (`surface::presentation`): Form surfaces use it when the layout class is Phone, the density is Controller, or the density is Touch and the window is under `surface-touch-popup-min-h` (900) tall. Otherwise they are a popup.

**Short windows.** Under `surface-short-h` (600) tall, the footer's actions move into the header and the footer row disappears, so a 480px landscape handheld keeps most of its height for the body.

**Keyboard avoidance.** The host reports the on-screen keyboard height (`HostState::keyboard_inset`). While it is above zero: the footer and hint bar hide, the body ends in a spacer as tall as the keyboard so the last field can scroll clear, the header shrinks to `surface-header-h-compact` (48) if the keyboard covers more than 40% of the window, and the focused field is scrolled into the part of the window still visible (`scroll_to_reveal`, 16px margin). Back stays reachable.

**Back** is the header button, the B button and Esc; all three do the same thing.
