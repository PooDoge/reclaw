//! The keyboard fallback: arrows, Enter, Escape and a few more map onto the same actions as the
//! pad, so Deck mode can be driven (and tested) without one.
use freya::prelude::*;
use reclaw_input::{Action, Button, Direction};

use super::key_holds::button_for_key;

pub(super) fn key_action(key: &Key, shift: bool) -> Option<Action> {
    Some(match key {
        Key::Named(NamedKey::ArrowUp) => Action::Navigate(Direction::Up),
        Key::Named(NamedKey::ArrowDown) => Action::Navigate(Direction::Down),
        Key::Named(NamedKey::ArrowLeft) => Action::Navigate(Direction::Left),
        Key::Named(NamedKey::ArrowRight) => Action::Navigate(Direction::Right),
        Key::Named(NamedKey::Enter) => Action::Confirm,
        Key::Named(NamedKey::Escape) => Action::Back,
        Key::Named(NamedKey::PageUp) => Action::PageUp,
        Key::Named(NamedKey::PageDown) => Action::PageDown,
        Key::Named(NamedKey::Tab) if shift => Action::QuickAccess,
        Key::Named(NamedKey::Tab) => Action::MainMenu,
        Key::Character(c) if c == "[" => Action::PrevSection,
        Key::Character(c) if c == "]" => Action::NextSection,
        Key::Character(c) if c == "o" => Action::Options,
        _ => return None,
    })
}

/// The pad button that X or Y on the keyboard stands for, which can be held for a notification's controls.
/// Not with Ctrl, Alt or Super: those are shortcuts, not buttons.
pub(super) fn hold_button(key: &Key, modifiers: Modifiers) -> Option<Button> {
    if modifiers.intersects(Modifiers::CONTROL | Modifiers::ALT | Modifiers::META) {
        return None;
    }
    match key {
        Key::Character(c) => button_for_key(&c.to_lowercase()),
        _ => None,
    }
}

/// While a text box has the keyboard only Enter and Escape are ours; every other key is typing.
pub(super) fn is_ours_while_typing(action: Action) -> bool {
    matches!(action, Action::Confirm | Action::Back)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arrows_enter_escape() {
        assert_eq!(key_action(&Key::Named(NamedKey::ArrowLeft), false), Some(Action::Navigate(Direction::Left)));
        assert_eq!(key_action(&Key::Named(NamedKey::Enter), false), Some(Action::Confirm));
        assert_eq!(key_action(&Key::Named(NamedKey::Tab), true), Some(Action::QuickAccess));
        assert_eq!(key_action(&Key::Character("z".into()), false), None);
    }

    #[test]
    fn x_and_y_are_hold_keys_unless_a_shortcut_modifier_is_down() {
        let x = Key::Character("x".into());
        assert_eq!(hold_button(&x, Modifiers::empty()), Some(Button::West));
        assert_eq!(hold_button(&Key::Character("Y".into()), Modifiers::SHIFT), Some(Button::North));
        assert_eq!(hold_button(&x, Modifiers::CONTROL), None);
        assert_eq!(hold_button(&Key::Named(NamedKey::Enter), Modifiers::empty()), None);
    }

    #[test]
    fn typing_only_yields_to_enter_and_escape() {
        assert!(is_ours_while_typing(Action::Confirm));
        assert!(is_ours_while_typing(Action::Back));
        assert!(!is_ours_while_typing(Action::Navigate(Direction::Left)));
        assert!(!is_ours_while_typing(Action::Options));
    }
}
