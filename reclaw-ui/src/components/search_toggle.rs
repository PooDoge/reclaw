use freya::{animation::*, prelude::*};

use super::{CLEAR, hoverable, pointer_cursor};
use crate::{metrics::*, prelude::*};

/// How long the box takes to open or close: the design system's 240 ms collapse, expo-out like its drawers.
const SLIDE_MS: u64 = 240;

/// The bar's paint layer, relative to its parent. Freya paints by depth (each element is a layer above its parent), so a plain
/// sibling's deeper text would paint over the open bar. This is above any page's depth and below `Layer::Overlay` (+2047), where
/// dialogs and menus are.
const BAR_LAYER: i16 = 1000;

/// A search box that is a square magnifier button until it is pressed, then slides open to the left over whatever is beside it.
/// Its own size never changes, so nothing around it moves. The field inside is always there: pressing the button, or reaching it
/// with Tab, focuses it, and the owner opens the box on that focus and closes it when focus goes (see `desktop::search`). While
/// open the magnifier sends the search, as Enter does. A dot on the closed button says a search is running.
#[derive(Clone, PartialEq)]
pub struct SearchToggle {
    open: bool,
    value: State<String>,
    a11y: AccessibilityId,
    placeholder: String,
    /// The width when open; closed it is `size` square.
    expanded: f32,
    size: f32,
    active: bool,
    animated: bool,
    on_submit: Option<EventHandler<String>>,
    key: DiffKey,
}

impl KeyExt for SearchToggle {
    fn write_key(&mut self) -> &mut DiffKey {
        &mut self.key
    }
}

impl SearchToggle {
    pub fn new(value: State<String>, a11y: AccessibilityId) -> Self {
        Self {
            open: false,
            value,
            a11y,
            placeholder: "Search".into(),
            expanded: 320.,
            size: 32.,
            active: false,
            animated: true,
            on_submit: None,
            key: DiffKey::None,
        }
    }

    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    pub fn expanded(mut self, width: f32) -> Self {
        self.expanded = width;
        self
    }

    /// The button's side, and the open box's height.
    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// A search is running here: the closed button carries a dot.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// Off when motion is reduced: the box opens and closes at once.
    pub fn animated(mut self, animated: bool) -> Self {
        self.animated = animated;
        self
    }

    pub fn on_submit(mut self, handler: impl Into<EventHandler<String>>) -> Self {
        self.on_submit = Some(handler.into());
        self
    }
}

impl Component for SearchToggle {
    fn render_key(&self) -> DiffKey {
        self.key.clone().or(self.default_key())
    }

    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let hovering = use_state(|| false);
        let ms = if self.animated { SLIDE_MS } else { 1 };
        let anim = use_animation_transition(self.open, move |from: bool, to: bool| {
            AnimNum::new(if from { 1. } else { 0. }, if to { 1. } else { 0. }).time(ms).ease(Ease::Out).function(Function::Expo)
        });
        // Focus the field when the box opens by any other way than pressing it (pressing focuses it already).
        let a11y = self.a11y;
        use_side_effect_with_deps(&self.open, move |open| {
            if *open {
                a11y.request_focus();
            }
        });

        let k = anim.read().value().clamp(0., 1.);
        let (size, value) = (self.size, self.value);
        let width = size + (self.expanded.max(size) - size) * k;
        let opened = self.open || k > 0.01;

        let on_submit = self.on_submit.clone();
        let open = self.open;
        let magnifier = rect()
            .width(Size::px(size))
            .height(Size::px(size))
            .center()
            .a11y_role(AccessibilityRole::Button)
            .a11y_alt(if open { "Search" } else { "Open search" })
            .maybe(open, |el| {
                el.on_press(move |_| {
                    if let Some(on_submit) = &on_submit {
                        on_submit.call(value.peek().clone());
                    }
                })
            })
            .child(icon(IconName::Search, 16., if open || hovering() { t.ink } else { t.ink_muted }));

        let colors = InputColorsThemePartial {
            background: Some(CLEAR.into()),
            focus_background: Some(CLEAR.into()),
            border_fill: Some(CLEAR.into()),
            focus_border_fill: Some(CLEAR.into()),
            color: Some(t.ink.into()),
            placeholder_color: Some(t.ink_subtle.into()),
        };
        let layout = InputLayoutThemePartial {
            corner_radius: Some(CornerRadius::new_all(RADIUS_MD).into()),
            padding: Some(Gaps::new(0., SPACE_1, 0., SPACE_4).into()),
        };
        let on_submit = self.on_submit.clone();
        let field = Input::new(value)
            .a11y_id(self.a11y)
            .placeholder(if opened { self.placeholder.clone() } else { String::new() })
            .width(Size::fill())
            .height(Size::px(size))
            .trailing(pointer_cursor(magnifier))
            .theme_colors(colors)
            .theme_layout(layout)
            .map(on_submit, |input, handler| input.on_submit(handler));

        let border = if self.open {
            t.accent
        } else if hovering() {
            t.line_strong
        } else {
            t.line
        };
        let bar = rect()
            .position(Position::new_absolute().top(0.).left(size - width))
            .layer(Layer::Relative(BAR_LAYER))
            .width(Size::px(width))
            .height(Size::px(size))
            .overflow(Overflow::Clip)
            .corner_radius(RADIUS_MD)
            .background(if opened || hovering() { t.bg_raised } else { t.bg_panel })
            .border(Border::new().fill(border).width(if self.open { 2. } else { 1. }).alignment(BorderAlignment::Inner))
            .maybe(k > 0.01, |el| el.shadow(Shadow::new().blur(16.).y(4.).color(Color::from_argb((60. * k) as u8, 0, 0, 0))))
            .child(field);

        // The dot: a search is running and the box is closed, so the list below is narrowed by something not on screen.
        let dot = (self.active && !opened).then(|| {
            rect()
                .position(Position::new_absolute().top(1.).left(size - 9.))
                .layer(Layer::Relative(BAR_LAYER + 1))
                .width(Size::px(8.))
                .height(Size::px(8.))
                .corner_radius(4.)
                .background(t.accent)
                .border(Border::new().fill(t.bg_nav).width(1.5).alignment(BorderAlignment::Outer))
        });

        let slot = rect().width(Size::px(size)).height(Size::px(size)).child(bar).maybe_child(dot);
        hoverable(slot, hovering)
    }
}
