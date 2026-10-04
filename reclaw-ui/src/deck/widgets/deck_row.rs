use std::borrow::Cow;

use freya::prelude::*;

use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// One list row in a panel: icon, label, accent bar when focused (no scale: rows stay aligned).
#[derive(Clone, PartialEq)]
pub struct DeckRow {
    pub icon: IconName,
    pub label: Cow<'static, str>,
    pub current: bool,
    pub focused: bool,
    pub on_press: EventHandler<()>,
}

impl Component for DeckRow {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let on_press = self.on_press.clone();
        let fg = if self.focused || self.current { t.ink } else { t.ink_muted };
        rect()
            .horizontal()
            .cross_align(Alignment::Center)
            .spacing(SPACE_4)
            .width(Size::fill())
            .height(Size::px(DECK_ROW_H))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .background(if self.focused { t.bg_raised } else { crate::components::CLEAR })
            .border(
                Border::new()
                    .fill(if self.focused { t.accent } else { crate::components::CLEAR })
                    .width(BorderWidth { left: 4., ..Default::default() })
                    .alignment(BorderAlignment::Inner),
            )
            .child(icon(self.icon, 24., fg))
            .child(TypeStyle::DeckBody.text(self.label.clone(), fg))
            .on_press(move |_| on_press.call(()))
    }
}
