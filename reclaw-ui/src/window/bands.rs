use freya::prelude::*;

use super::{
    command::WindowCommand,
    platform::use_maximized,
    resize::{Edge, THICKNESS, bands},
};

/// The resize bands of a window with no border: invisible strips along its edges that show the resize
/// cursor and hand a press to the window manager (as a [`WindowCommand::BeginResize`]). Freya's own
/// bands, in the borderless plugin, are switched off because they were placed from the window's physical
/// size and so missed the right and bottom edges on a scaled display (see `resize`).
///
/// `size` is the window's size in layout units, as the shell measures it.
#[derive(Clone, PartialEq)]
pub struct ResizeBands {
    size: (f32, f32),
    attached: bool,
    on_command: EventHandler<WindowCommand>,
}

impl ResizeBands {
    pub fn new(size: (f32, f32), attached: bool, on_command: EventHandler<WindowCommand>) -> Self {
        Self { size, attached, on_command }
    }
}

fn cursor_of(edge: Edge) -> CursorIcon {
    match edge {
        Edge::North | Edge::South => CursorIcon::NsResize,
        Edge::West | Edge::East => CursorIcon::EwResize,
        Edge::NorthWest | Edge::SouthEast => CursorIcon::NwseResize,
        Edge::NorthEast | Edge::SouthWest => CursorIcon::NeswResize,
    }
}

impl Component for ResizeBands {
    fn render(&self) -> impl IntoElement {
        let maximized = use_maximized(self.attached);
        // A maximized or fullscreen window has no edge to drag.
        let list = if maximized() { Vec::new() } else { bands(self.size, THICKNESS) };
        let elements = list.into_iter().map(|b| {
            let on_command = self.on_command.clone();
            rect()
                .position(Position::new_global().top(b.top).left(b.left))
                .width(Size::px(b.width))
                .height(Size::px(b.height))
                .cursor(cursor_of(b.edge))
                // The press is the band's alone: the corner band lies over the title bar's Close button, and a
                // refused resize must not close the window instead.
                .on_pointer_down(move |e: Event<PointerEventData>| {
                    e.stop_propagation();
                    on_command.call(WindowCommand::BeginResize(b.edge));
                })
                .on_press(|e: Event<PressEventData>| e.stop_propagation())
                .into_element()
        });
        // Over everything, and no room of its own: the bands are placed from the window's corner.
        rect().layer(Layer::Overlay).width(Size::px(0.)).height(Size::px(0.)).children(elements)
    }
}
