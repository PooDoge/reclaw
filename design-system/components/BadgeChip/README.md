A README's status badge (shields-style) as two parts: the label on `bg-raised`, the message on the badge's own color. Rust: `reclaw_ui::readme::BadgeChip`; the parsing of the badge's address into label, message and color is `reclaw_media::readme` (pure, tested).

Drawn natively because the toolkit's SVG drawing has no fonts, so a fetched badge image would show without its text. Message text is white, or near-black when the color is light (`wants_dark_text`). Pressable when the README wrapped the badge in a link; the link opens in the system browser, never inside the app.

Only badges whose address says its label, message and color are drawn this way. A badge that doesn't (a dynamic one) is a link to its address.
