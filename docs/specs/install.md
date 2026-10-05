# Installing

- last-verified: 2026-10-05
- owner-paths: reclaw-ui/src/settings/location.rs, reclaw-ui/src/desktop/dialogs/install.rs, reclaw-ui/src/components/install_dialog.rs, reclaw-ui/src/deck/pages/install.rs, reclaw-ui/src/deck/state/install.rs, reclaw-ui/src/deck/app/text_boxes.rs, reclaw-ui/tests/ui/desktop_surfaces.rs

What pressing Install asks, and where an app goes. (The installer itself is specified below as it is built.)

## The form

Both interfaces ask the same three things: **Install location**, **Create desktop shortcut** (on by default) and **Keep pre-release
builds** (off). There is no file to choose: Quiver downloads a prebuilt release and so does Reclaw. (An earlier version asked for "your
own game file" and kept Install disabled until one was picked; that step was an invention, not Quiver's behaviour, and is gone.)

## Where an app goes

The location box starts, **every time the form opens**, from Settings > Library > *Default install location*; when that is empty it
starts from `~/Reclaw/Apps` (`settings::default_install_location`). The Deck's Install page is seeded when it is shown
(`DeckState::take_text_seeds`), and the Deck's Settings text boxes are seeded the same way from what is stored, so what the page
shows is what the next install uses. An empty box at submit time means the default as well (`resolve_install_location`). Each app
goes in its own folder inside the location, named by the catalog's `folderName`.

## Tests

`settings::location` unit tests (fallback, trim, blank, per-app key not confused); `deck/state/tests/pages.rs` (a fresh draft, the seed
follows the setting each time the page opens, no file step, Install reachable immediately).
