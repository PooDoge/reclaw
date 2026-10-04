use freya::prelude::*;

use crate::nav::{LayerId, Nav, use_nav};

/// One dialog that can be open: its data, plus the navigation layer it holds while open. Back,
/// Escape and the mouse's back button close the layer first, so the dialog closes before the page
/// does; [`get`](Self::get) reports a dialog whose layer was closed that way as closed.
///
/// `Copy`, like the `State` inside: take it from the frame's context and move it into handlers.
pub(in crate::desktop) struct Slot<T: 'static> {
    state: State<Option<(T, LayerId)>>,
    nav: Nav,
}

impl<T> Clone for Slot<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Slot<T> {}

impl<T> PartialEq for Slot<T> {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}

impl<T: Clone + 'static> Slot<T> {
    /// Hooks: call once, from the frame.
    pub fn use_new() -> Self {
        Self { state: use_state(|| None), nav: use_nav() }
    }

    /// Show the dialog with `value`, replacing one that is already open.
    pub fn open(&self, value: T) {
        self.close();
        let layer = self.nav.open_layer();
        let mut state = self.state;
        state.set(Some((value, layer)));
    }

    /// Change the data of the open dialog (a menu moving into a submenu) without taking a new layer.
    pub fn update(&self, value: T) {
        let mut state = self.state;
        let layer = self.state.peek().as_ref().map(|(_, layer)| *layer);
        match layer {
            Some(layer) => state.set(Some((value, layer))),
            None => self.open(value),
        }
    }

    pub fn close(&self) {
        let mut state = self.state;
        if let Some((_, layer)) = state.peek().as_ref() {
            self.nav.close_layer(*layer);
        }
        state.set(None);
    }

    /// The open dialog's data, or `None`. Subscribes the caller to opening, closing and Back.
    pub fn get(&self) -> Option<T> {
        let (value, layer) = self.state.read().clone()?;
        self.nav.is_layer_open(layer).then_some(value)
    }

    /// Like [`get`](Self::get) for a handler: no subscription.
    pub fn peek(&self) -> Option<T> {
        let (value, layer) = self.state.peek().clone()?;
        self.nav.is_layer_open(layer).then_some(value)
    }
}
