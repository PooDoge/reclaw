The call-to-action control. Freya: `Button::new().filled()` (install, primary), `.outline()` (secondary), `.flat()` (ghost), themed through `theme_colors(ButtonColorsThemePartial{..})`.

Use `install` for the one verb that changes the game's state (Install, Play, Update); one per screen region, never two side by side. Use `primary` for non-game actions such as Check for updates. Labels are verbs in sentence case.

Consumer provides: label child, optional Lucide icon, `on_press`. Pointer height 32 (44 for the hero action); at touch density use `size: touch`, 48px tall, never below `target-min`.

Don't: color a button by game status; use `danger` anywhere but a confirmed destructive step; disable the install button without saying why in an adjacent badge.
