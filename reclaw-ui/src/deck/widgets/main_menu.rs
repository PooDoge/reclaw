use freya::prelude::*;
use reclaw_input::FocusId;

use super::deck_row::DeckRow;
use crate::{
    deck::{MAIN_MENU, MainMenuEntry, Section, ids},
    metrics::*,
    prelude::*,
    typography::TypeStyle,
};

fn menu_label(entry: MainMenuEntry) -> (&'static str, IconName) {
    match entry {
        MainMenuEntry::Section(Section::Library) => ("Library", IconName::Library),
        MainMenuEntry::Section(Section::Catalog) => ("Catalog", IconName::Catalog),
        MainMenuEntry::Section(Section::Downloads) => ("Downloads", IconName::Queue),
        MainMenuEntry::Section(Section::Mods) => ("Mods", IconName::Mods),
        MainMenuEntry::Settings => ("Settings", IconName::Settings),
        MainMenuEntry::SwitchToDesktop => ("Switch to desktop mode", IconName::Desktop),
    }
}

/// Left panel: sections, settings, and the way out to desktop mode.
#[derive(Clone, PartialEq)]
pub struct MainMenu {
    section: Section,
    focus: FocusId,
    ring_visible: bool,
    on_click: EventHandler<FocusId>,
}

impl MainMenu {
    pub fn new(section: Section, focus: FocusId, ring_visible: bool, on_click: EventHandler<FocusId>) -> Self {
        Self { section, focus, ring_visible, on_click }
    }
}

impl Component for MainMenu {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let rows = MAIN_MENU.iter().enumerate().map(|(i, entry)| {
            let (label, icon) = menu_label(*entry);
            let id = ids::menu(i);
            let on_click = self.on_click.clone();
            DeckRow {
                icon,
                label: label.into(),
                current: *entry == MainMenuEntry::Section(self.section),
                focused: self.ring_visible && self.focus == id,
                on_press: EventHandler::new(move |_| on_click.call(id)),
            }
            .into_element()
        });
        rect()
            .vertical()
            .width(Size::fill())
            .child(
                rect()
                    .height(Size::px(DECK_TABS_H))
                    .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
                    .main_align(Alignment::Center)
                    .child(TypeStyle::DeckHeading.text("Reclaw", t.ink)),
            )
            .children(rows)
    }
}
