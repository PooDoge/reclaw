Button prompts along the bottom safe zone. Rust: `reclaw_ui::deck::HintBar`.

Hints are contextual (Home: Select, Previous, Next, Quick access, Menu; Game page: Select, Library, Manage, Quick access; overlays: Select, Close). Glyphs come from `ActionMap::glyph(action, kind)`, so a rebinding changes the bar. After keyboard or mouse use the bar shows keycaps instead, and hints with no default key are hidden. Under Steam's virtual Xbox pad the glyphs read Xbox whatever hardware is in hand.
