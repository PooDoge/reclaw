use freya::prelude::*;

use super::Slot;
use crate::{
    desktop::use_desktop_ui,
    surface::{MenuEntry, MenuLevelView, MenuPlacement, MenuState, ModalMenu, Outcome, Presentation, SurfaceKind, presentation},
};

/// A list of options to choose one from, opened by a settings row: a popover at the press point under
/// a pointer, centered over a darkened screen on touch.
#[derive(Clone)]
pub struct OpenPicker {
    pub menu: MenuState<usize>,
    pub at: (f32, f32),
    /// Called with the index of the option chosen.
    pub on_pick: EventHandler<usize>,
}

impl OpenPicker {
    /// A picker titled `title` listing `labels`, with the cursor on `selected`.
    pub fn new(title: &str, labels: Vec<String>, selected: usize, at: (f32, f32), on_pick: EventHandler<usize>) -> Self {
        let mut menu = MenuState::new(title, labels.into_iter().enumerate().map(|(i, label)| MenuEntry::action(label, i)).collect());
        for _ in 0..selected {
            menu.navigate(reclaw_input::Direction::Down);
        }
        Self { menu, at, on_pick }
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct PickerView {
    pub picker: Slot<OpenPicker>,
}

impl Component for PickerView {
    fn render(&self) -> impl IntoElement {
        let ui = use_desktop_ui();
        let env = *ui.env.read();
        let Some(OpenPicker { menu, at, .. }) = self.picker.get() else { return rect().into_element() };
        let placement = match presentation(SurfaceKind::Menu, env.surface()) {
            Presentation::Anchored => MenuPlacement::Anchored { x: at.0, y: at.1 },
            _ => MenuPlacement::Centered,
        };
        let picker = self.picker;
        let dismiss = EventHandler::new(move |()| picker.close());
        let pick = EventHandler::new(move |(level, index): (usize, usize)| {
            let Some(mut open) = picker.peek() else { return };
            match open.menu.pick(level, index) {
                Outcome::Chose(i) => {
                    picker.close();
                    open.on_pick.call(i);
                }
                Outcome::Closed => picker.close(),
                Outcome::Moved | Outcome::None => {}
            }
        });
        ModalMenu::new(MenuLevelView::from_state(&menu), placement, env.window, env.density, pick, dismiss).into_element()
    }
}
