use freya::prelude::*;

use super::{PressHandler, hoverable, pointer_cursor, status_tone};
use crate::{metrics::*, prelude::*, typography::TypeStyle};

/// Left-rail library entry. Selected: raised fill plus a 3px accent bar, so selection never rests
/// on fill alone. The trailing dot is a secondary cue; the state's word is in the hero's badge.
///
/// Custom rather than `SideBarItem`: that component's active state is route-driven, and the
/// library selection is plain app state.
#[derive(Clone, PartialEq)]
pub struct LibraryRow {
    game: GameEntry,
    selected: bool,
    density: Density,
    on_press: Option<PressHandler>,
    key: DiffKey,
}

impl KeyExt for LibraryRow {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl LibraryRow {
    pub fn new(game: GameEntry) -> Self {
        Self {
            game,
            selected: false,
            density: Density::Pointer,
            on_press: None,
            key: DiffKey::None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }

    pub fn on_press(mut self, handler: impl Into<PressHandler>) -> Self {
        self.on_press = Some(handler.into());
        self
    }
}

impl Component for LibraryRow {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let active = self.selected || hovering();
        let fg = if active { t.ink } else { t.ink_muted };
        let (dot, _) = status_tone(&t, self.game.status);
        let hollow = self.game.status == AppStatus::Available;
        let text_style = match self.density {
            Density::Pointer => TypeStyle::Body,
            Density::Touch => TypeStyle::BodyTouch,
        };

        let row = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .spacing(SPACE_3)
            .width(Size::fill())
            .height(Size::px(self.density.row_height()))
            .padding(Gaps::new(0., SPACE_3, 0., SPACE_3))
            .corner_radius(RADIUS_MD)
            .background(if active { t.bg_raised } else { super::CLEAR })
            .maybe(self.selected, |el| {
                el.border(
                    Border::new()
                        .fill(t.accent)
                        .width(BorderWidth {
                            left: 3.,
                            ..Default::default()
                        })
                        .alignment(BorderAlignment::Inner),
                )
            })
            .child(
                rect()
                    .width(Size::px(20.))
                    .height(Size::px(20.))
                    .corner_radius(RADIUS_SM)
                    .background(if self.selected {
                        t.bg_panel
                    } else {
                        t.bg_raised
                    })
                    .border(
                        Border::new()
                            .fill(t.line_strong)
                            .width(1.)
                            .alignment(BorderAlignment::Inner),
                    ),
            )
            .child(
                rect().width(Size::flex(1.)).child(
                    text_style
                        .text(self.game.title.clone(), fg)
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                ),
            )
            .maybe(!self.game.version.is_empty(), |el| {
                el.child(TypeStyle::Mono.text(self.game.version.clone(), t.ink_subtle))
            })
            .child(
                rect()
                    .width(Size::px(8.))
                    .height(Size::px(8.))
                    .corner_radius(CornerRadius::new_all(4.))
                    .maybe(!hollow, |el| el.background(dot))
                    .maybe(hollow, |el| {
                        el.border(
                            Border::new()
                                .fill(t.ink_subtle)
                                .width(1.)
                                .alignment(BorderAlignment::Inner),
                        )
                    }),
            )
            .map(self.on_press.clone(), |el, handler| el.on_press(handler));

        pointer_cursor(hoverable(row, hovering))
    }

    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }
}
