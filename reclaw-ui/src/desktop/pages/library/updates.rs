//! The Updates section: every game with an update waiting, work in progress, a failure, or a result
//! from this run. A finished update stays here until the app restarts, so the user can see that it
//! happened and open the game to read what changed.
use freya::prelude::*;

use super::ctx::Ctx;
use crate::{
    activity::{IndicatorKind, SidebarEntry},
    components::{hoverable, indicator_look, pointer_cursor, progress_strip},
    metrics::*,
    nav::{Route, use_nav},
    prelude::*,
    typography::TypeStyle,
};

/// The section, or nothing when there is nothing to report.
pub(super) fn section(c: &Ctx) -> Option<Element> {
    if c.updates_section.is_empty() {
        return None;
    }
    let t = c.t;
    let rows = c.updates_section.iter().cloned().map(|entry| {
        let key = entry.game_id;
        UpdateRow { entry, key: DiffKey::None }.key(key).into_element()
    });
    Some(
        rect()
            .vertical()
            .spacing(2.)
            .width(Size::fill())
            .child(
                rect()
                    .padding(Gaps::new(SPACE_2, 0., SPACE_1, SPACE_1))
                    .child(TypeStyle::Eyebrow.text(format!("Updates ({})", c.updates_section.len()), t.ink_subtle)),
            )
            .children(rows)
            .into_element(),
    )
}

/// One game in the section: the icon for what is happening, its name, a line of detail and, while a
/// transfer with a known size runs, a thin bar along the bottom.
#[derive(Clone, PartialEq)]
struct UpdateRow {
    entry: SidebarEntry,
    key: DiffKey,
}

impl KeyExt for UpdateRow {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl Component for UpdateRow {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let (t, nav) = (use_reclaw(), use_nav());
        let hovering = use_state(|| false);
        let e = &self.entry;
        let (icon_name, color) = indicator_look(&t, e.indicator.kind);
        let id = e.game_id;
        let bar = e.indicator.is_active().then(|| {
            progress_strip(&t, e.indicator.kind, e.indicator.progress.filter(|_| e.indicator.kind != IndicatorKind::Installing), 3.)
        });
        let row = rect()
            .vertical()
            .width(Size::fill())
            .background(if hovering() { t.bg_raised } else { t.bg_panel })
            .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
            .corner_radius(RADIUS_MD)
            .overflow(Overflow::Clip)
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(format!("{}: {}", e.title, e.detail))
            .on_press(move |_| nav.open(Route::Game { id }))
            .child(
                rect()
                    .horizontal()
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_2)
                    .width(Size::fill())
                    .padding(Gaps::new(SPACE_2, SPACE_3, SPACE_2, SPACE_3))
                    .child(icon(icon_name, 16., color))
                    .child(
                        rect()
                            .vertical()
                            .width(Size::flex(1.))
                            .child(TypeStyle::Label.text(e.title.clone(), t.ink).max_lines(1).text_overflow(TextOverflow::Ellipsis))
                            .child(
                                TypeStyle::Meta
                                    .text(e.detail.clone(), if e.indicator.kind == IndicatorKind::Failed { t.danger } else { t.ink_muted })
                                    .max_lines(1)
                                    .text_overflow(TextOverflow::Ellipsis),
                            ),
                    ),
            )
            .maybe_child(bar);
        pointer_cursor(hoverable(row, hovering))
    }
}
