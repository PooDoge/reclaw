use freya::{animation::*, prelude::*};

use crate::{metrics::*, prelude::*};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PanelSide {
    Left,
    Right,
}

/// Overlay that slides in from an edge over the page. It never shifts the layout beneath, draws
/// the `scrim` token over the rest of the window and closes on a click outside.
#[derive(Clone, PartialEq)]
pub struct SlidePanel {
    side: PanelSide,
    open: bool,
    window: (f32, f32),
    on_close: EventHandler<()>,
    child: Element,
}

impl SlidePanel {
    pub fn new(side: PanelSide, open: bool, window: (f32, f32), on_close: EventHandler<()>, child: impl IntoElement) -> Self {
        Self { side, open, window, on_close, child: child.into_element() }
    }
}

impl Component for SlidePanel {
    fn render(&self) -> impl IntoElement {
        let t = use_reclaw();
        let anim = use_animation_transition(self.open, |from: bool, to: bool| {
            AnimNum::new(if from { 1. } else { 0. }, if to { 1. } else { 0. }).time(280).ease(Ease::Out).function(Function::Expo)
        });
        let k = anim.read().value();
        if !self.open && k <= 0.001 {
            return rect().into_element();
        }
        let (w, h) = self.window;
        let x = match self.side {
            PanelSide::Left => -DECK_PANEL_W * (1. - k),
            PanelSide::Right => w - DECK_PANEL_W * k,
        };
        let scrim = t.scrim;
        let faded = Color::from_argb((scrim.a() as f32 * k) as u8, scrim.r(), scrim.g(), scrim.b());
        let on_close = self.on_close.clone();

        // The root is itself absolute at the window origin: absolute children are placed relative to
        // their parent's origin, and a plain root would sit after the backdrop in the flow.
        // Layer::Overlay paints this above the whole page, including its own scaled focus frames.
        rect()
            .position(Position::new_absolute().top(0.).left(0.))
            .layer(Layer::Overlay)
            .width(Size::px(w))
            .height(Size::px(h))
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(0.))
                    .width(Size::px(w))
                    .height(Size::px(h))
                    .background(faded)
                    .on_press(move |_| on_close.call(())),
            )
            .child(
                rect()
                    .position(Position::new_absolute().top(0.).left(x))
                    .width(Size::px(DECK_PANEL_W))
                    .height(Size::px(h))
                    .background(t.bg_panel)
                    .border(Border::new().fill(t.line).width(1.).alignment(BorderAlignment::Inner))
                    .shadow(Shadow::new().blur(32.).y(12.).color(Color::from_argb(128, 0, 0, 0)))
                    .child(self.child.clone()),
            )
            .into_element()
    }
}
