Left-rail library entry (Steam's game list). Freya: `SideBarItem::new().on_press(..)` with active state from `use_is_active()`; the row is `rect().horizontal().content(Content::Flex)` with the name as `Size::flex(1.)`, then version and status dot at fixed size (the Content::Flex rule applies).

Selected: `bg-raised` fill plus a 3px accent bar on the left, so selection is not carried by fill alone. The dot is a secondary cue; the state's word appears in the hero's badge. Pointer height 32, touch 48.
