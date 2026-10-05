use freya::prelude::*;

use crate::{components::hoverable, metrics::*, prelude::*, typography::TypeStyle};

/// What sits on the right of a setting row.
#[derive(Clone, PartialEq)]
pub enum RowControl {
    Toggle(bool),
    /// A choice shown as its current value in a box with a chevron (opens a menu), or plain info.
    Value {
        text: String,
        opens_menu: bool,
    },
    /// A text box under the label. The state is the host's; the a11y id lets a gamepad press
    /// start typing by focusing it.
    Text {
        input: State<String>,
        placeholder: String,
        a11y: AccessibilityId,
        /// Show the text as dots (a token being pasted).
        secret: bool,
    },
    /// The row is the button.
    Action {
        danger: bool,
    },
}

/// Row heights, shared with the focus layout so what is drawn and what is focusable agree.
pub fn row_height(control_is_text: bool, density: Density) -> f32 {
    let base = match density {
        Density::Controller => DECK_SETTINGS_ROW_H,
        Density::Touch => 56.,
        Density::Pointer => 44.,
    };
    if control_is_text { base + 72. } else { base }
}

/// One line of a settings page: label (and description) left, control right, in a card.
#[derive(Clone, PartialEq)]
pub struct SettingRow {
    label: String,
    description: Option<String>,
    control: RowControl,
    focused: bool,
    disabled: bool,
    density: Density,
    on_press: Option<EventHandler<()>>,
    on_press_at: Option<EventHandler<(f32, f32)>>,
}

impl SettingRow {
    pub fn new(label: impl Into<String>, control: RowControl, density: Density) -> Self {
        Self {
            label: label.into(),
            description: None,
            control,
            focused: false,
            disabled: false,
            density,
            on_press: None,
            on_press_at: None,
        }
    }

    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_press(mut self, handler: impl Into<EventHandler<()>>) -> Self {
        self.on_press = Some(handler.into());
        self
    }

    /// Like [`on_press`](Self::on_press), told where in the window the press was, so a picker can
    /// open beside the row. A keyboard press has no position and reports the row's own corner (0, 0).
    pub fn on_press_at(mut self, handler: impl Into<EventHandler<(f32, f32)>>) -> Self {
        self.on_press_at = Some(handler.into());
        self
    }
}

fn switch(on: bool, t: &Reclaw) -> Rect {
    rect()
        .width(Size::px(52.))
        .height(Size::px(28.))
        .padding(3.)
        .corner_radius(14.)
        .background(if on { t.accent } else { t.bg_raised })
        .border(Border::new().fill(if on { t.accent } else { t.line_strong }).width(2.).alignment(BorderAlignment::Inner))
        .main_align(Alignment::Center)
        .horizontal()
        .child(rect().width(Size::px(if on { 25. } else { 3. })))
        .child(rect().width(Size::px(18.)).height(Size::px(18.)).corner_radius(9.).background(if on { t.on_accent } else { t.ink_muted }))
}

impl Component for SettingRow {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let controller = self.density == Density::Controller;
        let (label_style, meta_style) =
            if controller { (TypeStyle::DeckBody, TypeStyle::DeckMeta) } else { (TypeStyle::Body, TypeStyle::Meta) };
        let is_text = matches!(self.control, RowControl::Text { .. });
        let height = row_height(is_text, self.density);
        let danger = matches!(self.control, RowControl::Action { danger: true });
        let label_color = if self.disabled {
            t.ink_subtle
        } else if danger {
            t.danger
        } else {
            t.ink
        };

        let label = rect()
            .vertical()
            .width(Size::flex(1.))
            .child(label_style.text(self.label.clone(), label_color).max_lines(1).text_overflow(TextOverflow::Ellipsis))
            .maybe_child(
                self.description.clone().map(|d| meta_style.text(d, t.ink_muted).max_lines(1).text_overflow(TextOverflow::Ellipsis)),
            );

        let right: Option<Element> = match &self.control {
            RowControl::Toggle(on) => Some(switch(*on, &t).into_element()),
            RowControl::Value { text, opens_menu } => Some(
                rect()
                    .horizontal()
                    .cross_align(Alignment::Center)
                    .spacing(SPACE_3)
                    .padding(Gaps::new(SPACE_2, SPACE_4, SPACE_2, SPACE_4))
                    .background(if *opens_menu { t.bg_raised } else { crate::components::CLEAR })
                    .corner_radius(RADIUS_MD)
                    .child(label_style.text(text.clone(), if *opens_menu { t.ink } else { t.ink_muted }))
                    .maybe(*opens_menu, |el| el.child(icon(IconName::ChevronDown, 18., t.ink)))
                    .into_element(),
            ),
            RowControl::Text { .. } | RowControl::Action { .. } => None,
        };

        let top = rect()
            .horizontal()
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .width(Size::fill())
            .spacing(SPACE_4)
            .child(label)
            .maybe_child(right);

        let mut card = rect()
            .vertical()
            .main_align(Alignment::Center)
            .width(Size::fill())
            .height(Size::px(height))
            .padding(Gaps::new(0., SPACE_5, 0., SPACE_5))
            .background(if self.focused || hovering() { t.bg_raised } else { t.bg_panel })
            .border(
                Border::new()
                    .fill(if self.focused { t.accent } else { crate::components::CLEAR })
                    .width(DECK_FOCUS_RING)
                    .alignment(BorderAlignment::Inner),
            )
            .child(top);

        if let RowControl::Text { input, placeholder, a11y, secret } = &self.control {
            let mode = if *secret { InputMode::new_password() } else { InputMode::default() };
            card = card.child(rect().padding(Gaps::new(SPACE_2, 0., 0., 0.)).child(
                Input::new(*input).mode(mode).placeholder(placeholder.clone()).width(Size::fill()).a11y_id(*a11y).theme_colors(
                    InputColorsThemePartial {
                        background: Some(t.bg_base.into()),
                        focus_background: Some(t.bg_base.into()),
                        border_fill: Some(t.line_strong.into()),
                        focus_border_fill: Some(t.accent.into()),
                        color: Some(t.ink.into()),
                        placeholder_color: Some(t.ink_subtle.into()),
                    },
                ),
            ));
        }

        let card = card.map(self.on_press.clone().filter(|_| !self.disabled), |el, h| el.on_press(move |_| h.call(())));
        let card = card.map(self.on_press_at.clone().filter(|_| !self.disabled), |el, h| {
            el.on_press(move |e: Event<PressEventData>| {
                let at = match e.data() {
                    PressEventData::Mouse(m) => (m.global_location.x as f32, m.global_location.y as f32),
                    PressEventData::Touch(t) => (t.global_location.x as f32, t.global_location.y as f32),
                    PressEventData::Keyboard(_) => (0., 0.),
                };
                h.call(at);
            })
        });
        hoverable(card, hovering)
    }
}
