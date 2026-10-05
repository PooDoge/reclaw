# Surfaces: dialogs, pages and menus

A popup that is right on a desktop is wrong on a phone, in landscape with a keyboard up, or with a gamepad. Instead of each screen deciding, every popup is one of three **kinds**, and one table (`reclaw_ui::surface::presentation`) decides how each kind is presented for a given **form factor**. The screens describe content and actions once.

## The table

A form factor is a layout class (Wide, Compact, Phone), a density (Pointer, Touch, Controller) and the window height.

| kind | what it is | desktop (pointer, wide) | phone, short touch screen, Deck mode |
| --- | --- | --- | --- |
| **Form** | inputs, install, settings, properties | popup over a dimmed page | **full-screen page** with a Back button, scrolling body, footer actions |
| **Confirm** | a sentence and two buttons | popup | small centered card over a darkened screen |
| **Menu** | a list of choices that may cascade | anchored popover at the press point | **centered over a darkened screen**, focused row inverted, submenus open to the right |

A touch screen under 900px tall counts as short: a 1280x800 handheld or a phone in landscape gets full-screen forms; a portrait tablet keeps popups. Confirmations stay small everywhere because a full screen would be empty.

## Full-screen pages

The mobile settings pattern: **Back** and the title in a header, the fields in a scrolling column no wider than 720px, the actions in a footer, and in Deck mode the button hints along the bottom. Back is the header button, the B button and Esc, all the same action.

- **Short windows.** Under 600px tall the footer's actions move into the header, so a 480px landscape handheld keeps most of its height for the body.
- **On-screen keyboard.** The host tells the UI how tall the keyboard is. While it is up: the footer and hints hide (their actions are reachable again when it closes), the body gets a spacer so the last field can scroll clear, the header shrinks if the keyboard covers more than 40% of the window, and the focused field is scrolled into the part of the window still visible with a 16px margin. This is the "change the scroll position when the keyboard appears in landscape" behavior.
- **Two panes.** Settings use the full width on a window 900px wide or more: the section list on the left, the rows on the right, each scrolling by itself. Narrower windows show the list, then a section's rows.

## Menus

Modeled on Big Picture's option menu: the screen darkens, the app's name sits above, the menu is centered, and the **focused row is inverted** (light fill, dark text). `>` rows open a submenu in a second column to the right while the parent stays visible with its open row marked. On a window too narrow for two columns only the deepest level shows. Separators between groups are thick dark gaps. The menu for an app is: Add to favorites, Add to >, Manage >, Properties..., Cancel.

On a desktop the same menu is a small popover at the press point, with no scrim, dismissed by a click outside.

## Settings rows

One line high (68px at controller size), label and a one-line description on the left, control on the right: a switch, a value with a chevron that opens a centered menu, plain info, or the whole row as a button. A text row is 72px taller and holds an input. Entering a text box from a pad or keyboard clears it and returns the old text if nothing is typed, because the toolkit's input cannot place its caret from outside; a tap or click keeps the text.

## Entering Deck mode by hand

Deck mode starts automatically under a console session, but can always be entered to try it: **F10** toggles it anywhere; the desktop top bar has a **Deck mode** button; Settings > Interface can force Auto, Desktop or Deck. For testing, `RECLAW_LAYOUT`, `RECLAW_DENSITY` and `RECLAW_THEME` push the desktop into any form factor, **F9** shows a simulated on-screen keyboard (or `RECLAW_KEYBOARD=auto`, or `RECLAW_SIM_KEYBOARD=1` to raise it whenever a text box is focused).

## Not designed or built

An in-app on-screen keyboard (the OS provides one), a desktop Settings page, and a text box with a caret the interface can place.
